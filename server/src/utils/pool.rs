use legion::{Entity, EntityStore, World, world::SubWorld};
use utils::{
    arena::*,
    ids::{CoinTag, EnemyTag},
};

use crate::simulation::{
    resources::components::Active,
    systems::spawn::{spawn_coin_blank, spawn_enemy_blank},
};

pub struct Pool<T, Tag>(Arena<T, Tag>);

impl<T, Tag> Pool<T, Tag> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Arena::with_capacity(capacity))
    }

    #[inline]
    pub fn create(&mut self, value: T) -> Option<()> {
        self.0.init_slot(value)
    }

    #[inline]
    pub fn spawn(&mut self) -> Option<Id<Tag>> {
        self.0.acquire()
    }

    #[inline]
    pub fn recycle(&mut self, id: Id<Tag>) {
        self.0.release_index(id);
    }

    #[inline]
    pub fn get(&self, id: Id<Tag>) -> Option<&T> {
        self.0.get(id)
    }

    #[inline]
    pub fn get_mut(&mut self, id: Id<Tag>) -> Option<&mut T> {
        self.0.get_mut(id)
    }
}

pub trait HasPool<T, Tag> {
    fn get_pool(&self) -> &Pool<T, Tag>;
    fn get_pool_mut(&mut self) -> &mut Pool<T, Tag>;
}

#[macro_export]
macro_rules! impl_has_pool {
    ($storage:ty, $data:ty, $tag:ty, $field:ident) => {
        impl HasPool<$data, $tag> for $storage {
            #[inline]
            fn get_pool(&self) -> &Pool<$data, $tag> {
                &self.$field
            }
            #[inline]
            fn get_pool_mut(&mut self) -> &mut Pool<$data, $tag> {
                &mut self.$field
            }
        }
    };
}

pub struct PoolManager<Storage> {
    storage: Storage,
}

impl<Storage> PoolManager<Storage> {
    pub fn new(storage: Storage) -> Self {
        Self { storage }
    }

    pub fn acquire<Tag>(&mut self, world: &mut SubWorld) -> Option<(Id<Tag>, Entity)>
    where
        Tag: Send + Sync + 'static,
        Storage: HasPool<Entity, Tag>,
    {
        let pool = self.storage.get_pool_mut();
        let id = pool.spawn()?;

        let entity = match pool.get(id).copied() {
            Some(e) => e,
            None => {
                pool.recycle(id);
                return None;
            }
        };

        let mut entry = match world.entry_mut(entity) {
            Ok(e) => e,
            Err(_) => {
                pool.recycle(id);
                return None;
            }
        };

        match entry.get_component_mut::<Active>() {
            Ok(active) => active.0 = true,
            Err(_) => {
                pool.recycle(id);
                return None;
            }
        }

        Some((id, entity))
    }

    pub fn release<Tag>(&mut self, id: Id<Tag>, world: &mut SubWorld)
    where
        Tag: Send + Sync + 'static,
        Storage: HasPool<Entity, Tag>,
    {
        let pool = self.storage.get_pool_mut();
        let entity = match pool.get(id) {
            Some(e) => *e,
            None => {
                eprintln!("[PoolManager] release: Id invalide, entity introuvable");
                return;
            }
        };
        pool.recycle(id);

        if let Ok(mut entry) = world.entry_mut(entity) {
            if let Ok(active) = entry.get_component_mut::<Active>() {
                active.0 = false;
            }
        }
    }
}

const ENEMY_POOL_SIZE: usize = 100;
const COIN_POOL_SIZE: usize = 50;

pub struct GamePools {
    pub enemy: Pool<Entity, EnemyTag>,
    pub coin: Pool<Entity, CoinTag>,
}

impl GamePools {
    pub fn init(world: &mut World) -> Self {
        let mut enemy = Pool::<Entity, EnemyTag>::with_capacity(ENEMY_POOL_SIZE);
        for _ in 0..ENEMY_POOL_SIZE {
            enemy
                .create(spawn_enemy_blank(world))
                .expect("Pool enemy: capacité dépassée à l'init");
        }

        let mut coin = Pool::<Entity, CoinTag>::with_capacity(COIN_POOL_SIZE);
        for _ in 0..COIN_POOL_SIZE {
            coin.create(spawn_coin_blank(world))
                .expect("Pool coin: capacité dépassée à l'init");
        }
        Self { enemy, coin }
    }
}

impl_has_pool!(GamePools, Entity, EnemyTag, enemy);
impl_has_pool!(GamePools, Entity, CoinTag, coin);

#[cfg(test)]
mod pool_manager_tests {
    use super::*;
    use legion::*;

    #[test]
    fn test_pool_manager_enemy_lifecycle() {
        let mut world = World::default();
        let pools = GamePools::init(&mut world);
        let mut pool_manager = PoolManager::new(pools);

        let query = <&mut Active>::query();

        // 💡 On extrait le premier SubWorld dans sa propre variable bien distincte
        let (mut unwrapped_world, _) = world.split_for_query(&query); //

        // On passe la référence mutable du SubWorld extrait (&mut SubWorld)
        let acquire_res = pool_manager.acquire::<EnemyTag>(&mut unwrapped_world);
        assert!(
            acquire_res.is_some(),
            "L'acquisition d'un ennemi de la pool a échoué"
        );

        let (id, entity) = acquire_res.unwrap();

        // Vérification avec la bonne variable
        let entry = unwrapped_world.entry_ref(entity).unwrap();
        let active = entry.get_component::<Active>().unwrap();
        assert!(active.0, "L'entité acquise devrait être Active(true)");

        // Recyclage (Release)
        pool_manager.release::<EnemyTag>(id, &mut unwrapped_world);

        // Vérification finale
        let entry = unwrapped_world.entry_ref(entity).unwrap();
        let active = entry.get_component::<Active>().unwrap();
        assert!(!active.0, "L'entité libérée devrait être Active(false)");
    }

    #[test]
    fn test_pool_manager_coin_lifecycle() {
        let mut world = World::default();
        let pools = GamePools::init(&mut world);
        let mut pool_manager = PoolManager::new(pools);

        let query = <&mut Active>::query();

        // 💡 Même chose ici
        let (mut unwrapped_world, _) = world.split_for_query(&query); //

        let acquire_res = pool_manager.acquire::<CoinTag>(&mut unwrapped_world);
        assert!(acquire_res.is_some(), "L'acquisition d'une pièce a échoué");

        let (id, entity) = acquire_res.unwrap();

        let entry = unwrapped_world.entry_ref(entity).unwrap();
        let active = entry.get_component::<Active>().unwrap();
        assert!(active.0);

        pool_manager.release::<CoinTag>(id, &mut unwrapped_world);

        let entry = unwrapped_world.entry_ref(entity).unwrap();
        let active = entry.get_component::<Active>().unwrap();
        assert!(!active.0);
    }
}
