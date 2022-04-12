pub fn node_idx_for_shard(shard_idx: usize, replica_idx: u8, replicas: u8, nodes: u8) -> usize {
    todo!();
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
}
