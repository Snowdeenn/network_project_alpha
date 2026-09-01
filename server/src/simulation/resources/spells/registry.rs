
use utils::spell_types::{Spell, SpellId, RawSpell};
use utils::ids::SpellTag;

pub struct SpellRegister {
    inner: utils::Arena<Spell, SpellTag>,
    string_to_id: std::collections::HashMap<String, SpellId>,
}

impl SpellRegister {
    pub fn init(file_path: &str) -> std::io::Result<Self> {
        tracing::info!(path = %file_path, "Chargement du registre de sorts...");

        let spell_config_file = std::fs::read(file_path).map_err(|e| {
        tracing::error!(path = %file_path, error = %e, "Impossible de lire le fichier de sorts");
        e
    })?;

        let raw_spells: Vec<RawSpell> =
            serde_json::from_slice(&spell_config_file).map_err(|e| {
                tracing::error!(path = %file_path, error = %e, "Erreur de désérialisation JSON");
                std::io::Error::new(std::io::ErrorKind::InvalidData, e)
            })?;

        let mut inner = utils::Arena::new();
        let mut string_to_id = std::collections::HashMap::new();

        for raw_spell in raw_spells {
            let (raw_spell_id, spell) = raw_spell.into_spell();
            let spell_arena_id = inner.insert(spell);
            string_to_id.insert(raw_spell_id, SpellId::from(spell_arena_id));
        }

        tracing::info!("Registre de sorts initialisé avec succès");

        Ok(Self { inner, string_to_id })
    }

    pub fn resolve_string(&self, str: &str) -> Option<&SpellId> {
        self.string_to_id.get(str)
    }

    pub fn get_spell(&self, spell_id: SpellId) -> Option<&Spell> {
        self.inner.get(*spell_id)
    }
}
