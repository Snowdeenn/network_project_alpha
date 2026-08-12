use crate::math::vec3::Vec3;
use crate::math::vec4::Vec4;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Mat4 {
    pub columns: [f32; 16],
}

impl Mat4 {
    pub fn zero() -> Mat4 {
        Mat4 {
            columns: [0.0f32; 16],
        }
    }

    pub fn identity() -> Mat4 {
        let mut mat4: Mat4 = Mat4::zero();
        mat4.columns[0] = 1.0;
        mat4.columns[5] = 1.0;
        mat4.columns[10] = 1.0;
        mat4.columns[15] = 1.0;

        mat4
    }

    pub fn multiply(&self, other: Mat4) -> Mat4 {
        let mut result: Mat4 = Mat4::zero();

        for i in 0..4 {
            // on parcourt les lignes
            for j in 0..4 {
                // on parcourt les colonnes
                let mut sum: f32 = 0.0f32;
                for k in 0..4 {
                    // on fait le produit scalaire
                    sum += self.columns[k * 4 + i] * other.columns[j * 4 + k];
                }
                result.columns[j * 4 + i] = sum;
            }
        }
        result
    }

    pub fn transform(&self, vec: Vec4) -> Vec4 {
        let mut result: [f32; 4] = [0.0f32; 4];
        let v = [vec.x, vec.y, vec.z, vec.w];

        for i in 0..4 {
            let mut sum: f32 = 0.0;
            for k in 0..4 {
                sum += self.columns[k * 4 + i] * v[k];
            }
            result[i] = sum;
        }
        Vec4::new(result[0], result[1], result[2], result[3])
    }

    pub fn translation(x: f32, y: f32, z: f32) -> Mat4 {
        let mut mat_translation: Mat4 = Mat4::identity();

        mat_translation.columns[12] = x;
        mat_translation.columns[13] = y;
        mat_translation.columns[14] = z;

        mat_translation
    }

    pub fn scale(x: f32, y: f32, z: f32) -> Mat4 {
        let mut mat_scale: Mat4 = Mat4::identity();

        mat_scale.columns[0] = x;
        mat_scale.columns[5] = y;
        mat_scale.columns[10] = z;

        mat_scale
    }

    pub fn rotation_z(angle: f32) -> Mat4 {
        let mut mat_rota_z: Mat4 = Mat4::identity();

        mat_rota_z.columns[0] = angle.cos();
        mat_rota_z.columns[1] = angle.sin();
        mat_rota_z.columns[4] = -angle.sin();
        mat_rota_z.columns[5] = angle.cos();

        mat_rota_z
    }

    pub fn rotation_x(angle: f32) -> Mat4 {
        let mut mat_rota_x: Mat4 = Mat4::identity();

        mat_rota_x.columns[5] = angle.cos();
        mat_rota_x.columns[6] = angle.sin();
        mat_rota_x.columns[9] = -angle.sin();
        mat_rota_x.columns[10] = angle.cos();

        mat_rota_x
    }

    pub fn rotation_y(angle: f32) -> Mat4 {
        let mut mat_rota_y: Mat4 = Mat4::identity();

        mat_rota_y.columns[0] = angle.cos();
        mat_rota_y.columns[2] = -angle.sin();
        mat_rota_y.columns[8] = angle.sin();
        mat_rota_y.columns[10] = angle.cos();

        mat_rota_y
    }

    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
        let mut mat_perspective: Mat4 = Mat4::zero();
        let f: f32 = 1.0 / (fov_y / 2.0).tan();

        mat_perspective.columns[0] = f / aspect;
        mat_perspective.columns[5] = f;
        mat_perspective.columns[10] = (far + near) / (near - far);
        mat_perspective.columns[11] = -1.0;
        mat_perspective.columns[14] = (2.0 * far * near) / (near - far);

        mat_perspective
    }

    pub fn orthographic(left: f32, right: f32, bot: f32, top: f32, near: f32, far: f32) -> Mat4 {
        let mut mat_ortho: Mat4 = Mat4::identity();

        mat_ortho.columns[0] = 2.0 / (right - left);
        mat_ortho.columns[5] = 2.0 / (top - bot);
        mat_ortho.columns[10] = -1.0 / (far - near);
        mat_ortho.columns[12] = -(right + left) / (right - left);
        mat_ortho.columns[13] = -(top + bot) / (top - bot);
        mat_ortho.columns[14] = -near / (far - near);

        mat_ortho
    }

    pub fn look_at(eye: Vec3, center: Vec3, up: Vec3) -> Mat4 {
        let mut view_mat: Mat4 = Mat4::identity();

        // -------- Calculs vecteur orthogonaux --------
        let forward: Vec3 = (eye - center).normalize();
        let right: Vec3 = up.cross(forward).normalize();
        let up_real: Vec3 = forward.cross(right);

        view_mat.columns[0] = right.x;
        view_mat.columns[1] = up_real.x;
        view_mat.columns[2] = forward.x;
        view_mat.columns[4] = right.y;
        view_mat.columns[5] = up_real.y;
        view_mat.columns[6] = forward.y;
        view_mat.columns[8] = right.z;
        view_mat.columns[9] = up_real.z;
        view_mat.columns[10] = forward.z;
        view_mat.columns[12] = -right.dot(eye);
        view_mat.columns[13] = -up_real.dot(eye);
        view_mat.columns[14] = -forward.dot(eye);

        view_mat
    }
}
