use derive_more::{ IntoIterator, AsRef };
use factorial::Factorial;
use itertools::Itertools;

use std::collections::{ HashMap };
use std::sync::{ Arc, RwLock };

use crate::{
    trace,
    route::Route,
};
use super::{
    SyncSystem,
    System,
    SystemPair,
    CurrentShortest,
};



#[derive(Debug, AsRef, IntoIterator)]
pub struct SystemHolder {
    #[into_iterator(owned, ref, ref_mut)]
    inner: HashMap<String, SyncSystem>,
}

impl SystemHolder {
    pub fn new() -> SystemHolder {
        SystemHolder {
            inner: HashMap::new(),
        }
    }

    pub fn get(&self, system_name: &String) -> &Arc<RwLock<System>> {
        self.inner.get(system_name).unwrap()
    }

    pub fn all_inter_systems_iter(&self) -> impl Iterator<Item=SystemPair> {
        self.inner.clone().into_values().combinations(2).map(SystemPair::new)
    }

    pub fn register_system(&mut self, system: &System) {
        self.inner.insert(
            system.name().to_string(), 
            Arc::new(RwLock::new(system.clone()))
        );
    }

    pub fn register_route(&mut self, route: &Route) {
        for system in route {
            self.register_system(system);
        }
    }
}

impl SystemHolder {
    pub fn permutation_size_hint(&self) -> Option<u128> {
        ((self.inner.len()-1) as u128).checked_factorial()
    }

    pub fn build_shortest_path(&self, feedback_step: usize) -> CurrentShortest {
        let system_from: &SyncSystem = &self.get(
            crate::CLI_ARGS.read().unwrap().start.name()
        );

        let mut systems: Vec<SyncSystem> = self.inner.clone().into_values().collect();

        let system_from_index = systems
            .iter()
            .position(|ss| ss.read().unwrap().name() == system_from.read().unwrap().name())
            .unwrap();
        systems.remove(system_from_index);

        let mut current_shortest = CurrentShortest::new();

        for (idx, sync_route) in systems.clone().into_iter().permutations(systems.len()).enumerate() {
            if idx.wrapping_rem(feedback_step) == 0 {
                crate::PROGRESS_HOLDER.write().unwrap().feedback(idx as u128);
            }

            let system_from_rlock = system_from.read().unwrap();
            let mut route_length: u64 = system_from_rlock
                .get_distance_to(&sync_route[0])
                .expect( &trace::string::error( format!("Distance from '{}' to '{}' not set",
                    system_from_rlock.name(),
                    sync_route[0].read().unwrap().name(),
                )));

            sync_route.iter().as_slice().windows(2).for_each(
                |window| match window {
                    [prev, next] => {
                        let prev_rlock = prev.read().unwrap();

                        let length_step: u64 = prev_rlock
                            .get_distance_to(&next)
                            .expect( &trace::string::error( format!("Distance from '{}' to '{}' not set",
                                prev_rlock.name(),
                                next.read().unwrap().name(),
                            )));

                        route_length += length_step;
                    },
                    _ => {
                        trace::error("Unexpected error: window size must be 2, got non 2 value");
                    }
                }
            );

            current_shortest.register(&sync_route, route_length);
        }

        // last report on 100%
        crate::PROGRESS_HOLDER.write().unwrap().feedback(self.permutation_size_hint().unwrap_or(u128::MAX));

        current_shortest
    }
}