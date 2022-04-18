use itertools::Itertools;


pub struct ShardDistribution {
    nodes: u8,
    replicas: u8,
    distribution: Vec<Vec<u8>>,
}

impl ShardDistribution {
    pub fn new(nodes: u8, replicas: u8) -> ShardDistribution {
 
        assert!(replicas <= nodes);

        let idxs: Vec<u8> = (0..nodes).collect();
        let distribution = idxs.into_iter().combinations(replicas.into()).collect_vec();


        ShardDistribution{nodes, replicas, distribution}
    }


    pub fn node_idx_for_shard(&self, shard_idx: usize, replica_idx: u8) -> u8 {

        let mod_shard_idx = shard_idx % self.distribution.len(); 
        self.distribution[mod_shard_idx][replica_idx as usize]
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

        let shard_distribution = ShardDistribution::new(setup.n_nodes, setup.n_replicas);

        for shard_idx in 0..setup.n_shards {
            for replica_idx in 0..setup.n_replicas {
                let node_idx =
                    shard_distribution.node_idx_for_shard(shard_idx, replica_idx);
                let replica_iden = ReplicaIdentity::new(shard_idx, replica_idx);
                res.nodes[node_idx as usize].push(replica_iden);
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
