pub fn node_idx_for_shard(shard_idx: usize, replica_idx: u8, replicas: u8, nodes: u8) -> usize {
    match nodes {
        0 => {
            panic!("Node count must be greater than zero.")
        }
        1 => 0,
        _ => match replicas {
            0 => {
                panic!("Replica count must be greater than zero.")
            }
            1 => shard_idx % nodes as usize,

            2 => match nodes {
                2 => replica_idx as usize,
                4 => {
                    let rep_0_node_idxs = vec![0, 0, 0, 1, 1, 2];
                    let rep_1_node_idxs = vec![1, 2, 3, 2, 3, 3];

                    let rep_node_idxs = vec![rep_0_node_idxs, rep_1_node_idxs];

                    let mod_shard_idx = shard_idx % 6;

                    rep_node_idxs[replica_idx as usize][mod_shard_idx]
                }
                _ => 0,
            },
            _ => 0,
        },
    }
}

#[derive(PartialEq)]
pub struct ReplicaIdentity {
    shard_idx: usize,
    replica_idx: u8,
}

impl ReplicaIdentity {
    fn new(shard_idx: usize, replica_idx: u8) -> ReplicaIdentity {
        ReplicaIdentity {
            shard_idx,
            replica_idx,
        }
    }
}

pub struct DistributionSetup {
    n_nodes: u8,
    n_replicas: u8,
    n_shards: usize,
}

#[cfg(test)]
mod tests {

    use super::*;

    const N_SHARDS: usize = 50;

    pub struct DistributionResult {
        nodes: Vec<Vec<ReplicaIdentity>>,
    }

    impl DistributionResult {
        fn new() -> DistributionResult {
            let nodes: Vec<Vec<ReplicaIdentity>> = Vec::new();
            DistributionResult { nodes }
        }
    }

    fn simulate_distribution(setup: DistributionSetup) -> DistributionResult {
        let mut res = DistributionResult::new();
        for node_idx in 0..setup.n_nodes {
            let replicas_in_node: Vec<ReplicaIdentity> = Vec::new();
            res.nodes.push(replicas_in_node);
        }

        for shard_idx in 0..setup.n_shards {
            for replica_idx in 0..setup.n_replicas {
                let node_idx =
                    node_idx_for_shard(shard_idx, replica_idx, setup.n_replicas, setup.n_nodes);
                let replica_iden = ReplicaIdentity::new(shard_idx, replica_idx);
                res.nodes[node_idx].push(replica_iden);
            }
        }
        res
    }

    #[test]
    fn nodes_1_replicas_1() {
        let n_nodes = 1;
        let n_replicas = 1;
        let n_shards = N_SHARDS;
        let setup = DistributionSetup {
            n_nodes,
            n_replicas,
            n_shards,
        };

        let res = simulate_distribution(setup);
        assert_eq!(res.nodes.len() as u8, n_nodes);

        for shard_idx in 0..n_shards {
            for replica_idx in 0..n_replicas {
                let entry = ReplicaIdentity::new(shard_idx, 0);
                assert!(res.nodes[0].contains(&entry));
            }
        }
    }

    #[test]
    fn nodes_2_replicas_1() {
        let n_nodes = 2;
        let n_replicas = 1;
        let n_shards = N_SHARDS;
        let setup = DistributionSetup {
            n_nodes,
            n_replicas,
            n_shards,
        };

        let res = simulate_distribution(setup);
        assert_eq!(res.nodes.len() as u8, n_nodes);

        for shard_idx in 0..n_shards {
            for replica_idx in 0..n_replicas {
                let entry = ReplicaIdentity::new(shard_idx, replica_idx);

                assert!(res.nodes[shard_idx % 2].contains(&entry));
            }
        }
    }

    #[test]
    fn nodes_4_replicas_2() {
        let n_nodes = 4;
        let n_replicas = 2;
        let n_shards = N_SHARDS;
        let setup = DistributionSetup {
            n_nodes,
            n_replicas,
            n_shards,
        };

        let rep_0_node_idxs = vec![0, 0, 0, 1, 1, 2];
        let rep_1_node_idxs = vec![1, 2, 3, 2, 3, 3];

        let rep_node_idxs = vec![rep_0_node_idxs, rep_1_node_idxs];

        let res = simulate_distribution(setup);
        assert_eq!(res.nodes.len() as u8, n_nodes);

        for shard_idx in 0..n_shards {
            for replica_idx in 0..n_replicas {
                let entry = ReplicaIdentity::new(shard_idx, replica_idx);

                let mod_shard_idx = shard_idx % 6;
                //  let node_idx = rep_node_idxs[replica_idx[mod_shard_idx];
                let node_idx = rep_node_idxs[replica_idx as usize][mod_shard_idx];
                assert!(res.nodes[node_idx].contains(&entry));
            }
        }
    }
}
