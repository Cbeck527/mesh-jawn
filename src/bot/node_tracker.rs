use meshtastic::protobufs::NodeInfo;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

const ONLINE_THRESHOLD_SECS: u32 = 7200; // 2 hours

/// Tracks nodes in the mesh network
pub struct NodeTracker {
    /// Our own node number
    my_node_num: Option<u32>,
    /// Map of node_num to NodeInfo
    nodes: HashMap<u32, NodeInfo>,
}

impl NodeTracker {
    pub fn new() -> Self {
        Self {
            my_node_num: None,
            nodes: HashMap::new(),
        }
    }

    /// Set our own node number (from MyNodeInfo)
    pub fn set_my_node_num(&mut self, num: u32) {
        self.my_node_num = Some(num);
    }

    /// Get our own node number
    pub fn my_node_num(&self) -> Option<u32> {
        self.my_node_num
    }

    /// Update or add a node to the tracker
    pub fn update_node(&mut self, node_info: NodeInfo) {
        let node_num = node_info.num;
        self.nodes.insert(node_num, node_info);
    }

    /// Get total number of nodes we know about
    pub fn total_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Get number of nodes heard within the last 2 hours
    pub fn online_nodes(&self) -> usize {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as u32;

        self.nodes
            .values()
            .filter(|node| {
                if node.last_heard == 0 {
                    return false;
                }
                let elapsed = now.saturating_sub(node.last_heard);
                elapsed <= ONLINE_THRESHOLD_SECS
            })
            .count()
    }

    /// Get a specific node by node number
    pub fn get_node(&self, node_num: u32) -> Option<&NodeInfo> {
        self.nodes.get(&node_num)
    }

    /// Get all nodes
    pub fn all_nodes(&self) -> impl Iterator<Item = &NodeInfo> {
        self.nodes.values()
    }
}

impl Default for NodeTracker {
    fn default() -> Self {
        Self::new()
    }
}
