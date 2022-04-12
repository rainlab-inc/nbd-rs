pub fn node_idx_for_shard(shard_idx: usize, replicas: usize, nodes: usize) -> usize {
    todo!()
}

#[cfg(test)]
mod tests {

    use crate::block::shard_distribution::node_idx_for_shard;
    #[test]
    fn node_1_copy_1() {
        for shard_idx in 0..50 {
            let node_idx = node_idx_for_shard(shard_idx, 1, 1);
            assert_eq!(node_idx, 0);
        }
    }

}
