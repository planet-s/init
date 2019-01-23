use std::collections::HashMap;

use generational_arena::Index;
use log::{error, warn};

use crate::dep_graph::DepGraph;
use crate::service::{Service, services};

const FS_SERVICE_DIR: &str = "file:/etc/init.d";

/// Main data structure for init, containing the main interface
/// for dealing with services
pub struct ServiceTree {
    graph: DepGraph<Service>
}

impl ServiceTree {
    pub fn new() -> ServiceTree {
        ServiceTree {
            graph: DepGraph::new()
        }
    }
    
    /// Push some services into the graph, and add their dependency nodes.
    /// Note that this does not start any services, only their metadata
    /// is inserted into the graph. Metadata for services is not manipulated
    pub fn push_services(&mut self, mut services: Vec<Service>) {
        self.graph.reserve(services.len());
        
        let services: HashMap<String, Index> = services.drain(..)
            .map(|service| (service.name.clone(), self.graph.insert(service)) )
            .collect();
        
        for parent in services.values() {
            let dependencies = self.graph.get(*parent)
                .expect("services were just added")
                .dependencies.clone();
            
            if let Some(ref dependencies) = dependencies {
                for dependency in dependencies.iter() {
                    match services.get(dependency) {
                        Some(child) => self.graph.dependency(*parent, *child)
                            .unwrap_or_else(|_| warn!("failed to add dependency") ),
                        // It's not a super big deal if a dependency doesn't exist
                        // I mean, it is, but IDK what to do in that situation
                        //   It's really a pkg problem at that point
                        //TODO: The dep really needs to be invalidated in some way
                        None => warn!("dependency not found: {}", dependency)
                    }
                }
            }
        }
    }
    
    /// WIP: This function attempts to run the start method on each service in the graph
    /// if it is not already running or starting.
    pub fn start_services(&mut self) {
        let resolved = self.graph.linear_resolve();
        
        for index /*group*/ in resolved.iter() {
            //for index in group.iter() {
                let service = self.graph.get_mut(*index)
                    // These should all exist, the resolver can only
                    // return indexes that are in the graph anyway
                    .expect("resolved service index did not exist");
                
                if !(service.state.is_starting() || service.state.is_online()) {
                    service.wait_method(&"start".to_string())
                        .unwrap_or_else(|err| { error!("error starting service '{}': {}", service.name, err) });
                    
                    if let Some(provides) = &service.provides {
                        /*
                        if provides.contains(&"display:".to_string()) {
                            crate::switch_stdio("display:1")
                                .unwrap_or_else(|err| {
                                    warn!("{}", err);
                                });
                        }*/
                        
                        if provides.contains(&"file:".to_string()) {
                            let fs_services = services(FS_SERVICE_DIR)
                                .unwrap_or_else(|err| {
                                    error!("error parsing service directory '{}': {}", FS_SERVICE_DIR, err);
                                    vec![]
                                });
                            
                            // This is surely a poor descision, should probably be
                            //   made on a per-service basis
                            /*
                            info!("setting cwd to 'file:'");
                            if let Err(err) = env::set_current_dir("file:") {
                                error!("failed to set cwd: {}", err);
                            }*/
                            
                            self.push_services(fs_services);
                            self.start_services();
                        }
                    }
                }
            //}
        }
    }
}