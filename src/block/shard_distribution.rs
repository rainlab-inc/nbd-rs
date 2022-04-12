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

    #[test]
    fn node_2_copy_1() {
        let mut alter_flag = true;
        for shard_idx in 0..50 {
            let node_idx = node_idx_for_shard(shard_idx, 1, 2);
            if alter_flag {
                assert_eq!(shard_idx, 0);
            } else {
                assert_eq!(shard_idx, 1);
            }
            alter_flag = !alter_flag;
        }
    }

    #[test]
    fn node_2_copy_2() {
        let mut alter_flag = true;
        let mut alter_count = 0;

        for shard_idx in 0..50 {
            let node_idx = node_idx_for_shard(shard_idx, 2, 2);
            if alter_flag {
                assert_eq!(shard_idx, 0);
            } else {
                assert_eq!(shard_idx, 1);
            }
            alter_count = alter_count + 1;
            if alter_count == 2 {
                alter_flag = !alter_flag;
                alter_count = 0;
            }
        }
    }

}
