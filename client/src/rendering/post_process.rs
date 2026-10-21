//! Extensible post-process registration and owned per-frame uniforms.
use prism::{
    RenderError, RenderResult,
    frame::v2::{PostProcessId, PostProcessSubmission},
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq)]
pub enum PostProcessValue {
    Scalar(f32),
    Vector(Vec<f32>),
}

#[derive(Clone, Debug)]
pub struct PostProcessRequest {
    pub effect: String,
    pub material: String,
    pub weight: f32,
    pub remaining_seconds: f32,
    pub parameters: HashMap<String, PostProcessValue>,
}

/// Values available to an effect's own uniform encoder. No central effect enum is required.
pub struct PostProcessInput<'a> {
    pub effect: &'a str,
    pub material: &'a str,
    pub weight: f32,
    pub remaining_seconds: f32,
    pub parameters: &'a HashMap<String, PostProcessValue>,
    pub viewport: [u32; 2],
}

impl PostProcessInput<'_> {
    pub fn aspect_ratio(&self) -> f32 {
        self.viewport[0] as f32 / self.viewport[1].max(1) as f32
    }
    pub fn scalar(&self, name: &str) -> Option<f32> {
        match self.parameters.get(name)? {
            PostProcessValue::Scalar(value) => Some(*value),
            _ => None,
        }
    }
    pub fn vector(&self, name: &str) -> Option<&[f32]> {
        match self.parameters.get(name)? {
            PostProcessValue::Vector(values) => Some(values),
            _ => None,
        }
    }
}

type Encoder = Box<dyn Fn(&PostProcessInput<'_>) -> Vec<u8>>;
struct Pass {
    id: PostProcessId,
    material: Option<String>,
    encode: Option<Encoder>,
    uniform: Option<Vec<u8>>,
    parameter_size: usize,
    enabled: bool,
}

#[derive(Default)]
pub(super) struct PostProcessRegistry {
    passes: Vec<Pass>,
    by_material: HashMap<String, usize>,
    missing: HashSet<String>,
}

impl PostProcessRegistry {
    pub fn check_material(&self, name: &str) -> RenderResult<()> {
        if name.is_empty() || self.by_material.contains_key(name) {
            return Err(RenderError::new(
                "post-process registration",
                "empty or already registered material name",
            ));
        }
        Ok(())
    }

    pub fn register<T: bytemuck::Pod + 'static>(
        &mut self,
        material: String,
        id: PostProcessId,
        encode: impl Fn(&PostProcessInput<'_>) -> T + 'static,
    ) {
        self.missing.remove(&material);
        self.by_material.insert(material.clone(), self.passes.len());
        self.passes.push(Pass {
            id,
            material: Some(material),
            encode: Some(Box::new(move |input| {
                bytemuck::bytes_of(&encode(input)).to_vec()
            })),
            uniform: None,
            parameter_size: std::mem::size_of::<T>(),
            enabled: true,
        });
    }

    pub fn add_manual(&mut self, id: PostProcessId, uniform: Vec<u8>) {
        let parameter_size = uniform.len();
        self.passes.push(Pass {
            id,
            material: None,
            encode: None,
            uniform: Some(uniform),
            parameter_size,
            enabled: true,
        });
    }

    fn pass_mut(&mut self, id: PostProcessId) -> RenderResult<&mut Pass> {
        self.passes
            .iter_mut()
            .find(|pass| pass.id == id)
            .ok_or_else(|| RenderError::new("post-process", "unknown pass identifier"))
    }

    pub fn enable(&mut self, id: PostProcessId, enabled: bool) -> RenderResult<()> {
        self.pass_mut(id)?.enabled = enabled;
        Ok(())
    }

    pub fn write<T: bytemuck::Pod>(&mut self, id: PostProcessId, uniform: T) -> RenderResult<()> {
        let pass = self.pass_mut(id)?;
        if std::mem::size_of::<T>() != pass.parameter_size {
            return Err(RenderError::new(
                "post-process uniform",
                "uniform size differs from the registered layout",
            ));
        }
        pass.uniform = Some(bytemuck::bytes_of(&uniform).to_vec());
        Ok(())
    }

    pub fn submissions(
        &mut self,
        requests: &[PostProcessRequest],
        viewport: [u32; 2],
    ) -> Vec<PostProcessSubmission> {
        for request in requests {
            if !self.by_material.contains_key(&request.material)
                && self.missing.insert(request.material.clone())
            {
                tracing::warn!(material = %request.material, effect = %request.effect, "Matériau de post-process non enregistré");
            }
        }
        let mut submissions = Vec::new();
        // Registration order is the composition order, as in the historical pass API.
        for pass in &self.passes {
            if !pass.enabled {
                continue;
            }
            if let Some(material) = &pass.material {
                for request in requests
                    .iter()
                    .filter(|request| request.material == *material && request.weight > 0.0)
                {
                    let parameters = pass.uniform.clone().unwrap_or_else(|| {
                        pass.encode.as_ref().unwrap()(&PostProcessInput {
                            effect: &request.effect,
                            material: &request.material,
                            weight: request.weight,
                            remaining_seconds: request.remaining_seconds,
                            parameters: &request.parameters,
                            viewport,
                        })
                    });
                    submissions.push(PostProcessSubmission {
                        effect: pass.id,
                        parameters,
                    });
                }
            } else {
                submissions.push(PostProcessSubmission {
                    effect: pass.id,
                    parameters: pass.uniform.clone().unwrap(),
                });
            }
        }
        submissions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(material: &str) -> PostProcessRequest {
        PostProcessRequest {
            effect: "custom effect".into(),
            material: material.into(),
            weight: 0.5,
            remaining_seconds: 1.0,
            parameters: HashMap::from([
                ("strength".into(), PostProcessValue::Scalar(0.25)),
                (
                    "tint".into(),
                    PostProcessValue::Vector(vec![0.0, 1.0, 0.0, 1.0]),
                ),
            ]),
        }
    }
    #[test]
    fn arbitrary_materials_encode_their_own_parameters_in_registration_order() {
        let mut registry = PostProcessRegistry::default();
        registry.register("custom/a".into(), PostProcessId(1), |input| {
            [
                input.scalar("strength").unwrap(),
                input.weight,
                input.aspect_ratio(),
                input.vector("tint").unwrap()[1],
            ]
        });
        registry.register("custom/b".into(), PostProcessId(2), |input| {
            input.remaining_seconds
        });
        let a = request("custom/a");
        let b = request("custom/b");
        let result = registry.submissions(&[b, a], [800, 400]);
        assert_eq!(
            result.iter().map(|s| s.effect).collect::<Vec<_>>(),
            vec![PostProcessId(1), PostProcessId(2)]
        );
        assert_eq!(
            bytemuck::cast_slice::<u8, f32>(&result[0].parameters),
            [0.25, 0.5, 2.0, 1.0]
        );
        assert!(registry.check_material("custom/a").is_err());
    }
    #[test]
    fn manual_pass_uniforms_are_owned_and_enable_disable_write_remain_dynamic() {
        let mut registry = PostProcessRegistry::default();
        registry.add_manual(PostProcessId(1), bytemuck::bytes_of(&0.25f32).to_vec());
        let first = registry.submissions(&[], [64, 64]);
        registry.write(PostProcessId(1), 0.75f32).unwrap();
        let second = registry.submissions(&[], [64, 64]);
        assert_eq!(
            bytemuck::pod_read_unaligned::<f32>(&first[0].parameters),
            0.25
        );
        assert_eq!(
            bytemuck::pod_read_unaligned::<f32>(&second[0].parameters),
            0.75
        );
        assert!(registry.write(PostProcessId(1), [1.0f32; 4]).is_err());
        registry.enable(PostProcessId(1), false).unwrap();
        assert!(registry.submissions(&[], [64, 64]).is_empty());
        registry.enable(PostProcessId(1), true).unwrap();
        assert_eq!(registry.submissions(&[], [64, 64]).len(), 1);
        assert!(registry.enable(PostProcessId(999), true).is_err());
    }
    #[test]
    fn missing_materials_skip_only_the_unknown_effect_and_can_be_registered_later() {
        let mut registry = PostProcessRegistry::default();
        assert!(registry.submissions(&[request("late")], [1, 1]).is_empty());
        assert_eq!(registry.missing.len(), 1);
        registry.register("late".into(), PostProcessId(3), |input| input.weight);
        assert!(registry.missing.is_empty());
        assert_eq!(registry.submissions(&[request("late")], [1, 1]).len(), 1);
    }
}
