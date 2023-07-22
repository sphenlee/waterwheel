use std::borrow::Borrow;

pub struct Rendezvous<Node> {
    nodes: Vec<Node>,
}

impl<Node> Default for Rendezvous<Node> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Node> Rendezvous<Node> {
    pub fn new() -> Self {
        Rendezvous { nodes: vec![] }
    }

    pub fn with_me(me: Node) -> Self {
        Rendezvous { nodes: vec![me] }
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    pub fn add_node(&mut self, node: Node) {
        self.nodes.push(node);
    }
}

impl<Node: AsRef<[u8]>> Rendezvous<Node> {
    fn score_for_node<Item>(node: &Node, item: &Item) -> u64
        where
            Item: AsRef<[u8]> + ?Sized,
    {
        let mut hasher = xxhash_rust::xxh3::Xxh3::default();
        hasher.update(node.as_ref());
        hasher.update(item.as_ref());
        hasher.digest()
    }

    pub fn node_for_item<Item>(&self, item: &Item) -> Option<&Node>
        where
            Item: AsRef<[u8]> + ?Sized,
    {
        let target = self
            .nodes
            .iter()
            .max_by_key(|node| Rendezvous::score_for_node(node, item));

        target
    }
}

impl<Node> Rendezvous<Node>
where Node: AsRef<[u8]>
{
    pub fn item_is_mine<Item, Me>(&self, me: &Me, item: &Item) -> bool
        where
            Item: AsRef<[u8]> + ?Sized,
            Node: Borrow<Me>,
            Me: PartialEq<Me> + ?Sized,
    {
        match self.node_for_item(item) {
            None => false,
            Some(item) => item.borrow() == me
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_rendezvous_empty() {
        let r = Rendezvous::<&str>::new();
        let t = r.node_for_item("ItemX");
        assert!(t.is_none());
    }

    #[test]
    fn test_rendezvous() {
        let mut r = Rendezvous::with_me("ServerA");
        r.add_node("ServerB");

        let t = r.node_for_item("ItemX").unwrap();
        assert_eq!(*t, "ServerA");

        let t = r.node_for_item("ItemY").unwrap();
        assert_eq!(*t, "ServerB");

        assert!(r.item_is_mine("ServerA", "ItemX"));
    }

    #[test]
    fn test_rendezvous_distribution() {
        let mut r = Rendezvous::with_me("ServerA".to_owned());
        r.add_node("ServerB".to_owned());

        let mut a = 0;
        let mut b = 0;
        for i in 0..1000 {
            let ab = r.node_for_item(&format!("node{i}")).unwrap();
            if ab == "ServerA" {
                a += 1;
            } else {
                b += 1;
            }
        }

        // sanity check!
        assert_eq!(a + b, 1000);
        // close enough to 50/50
        assert_eq!(a, 486);
        assert_eq!(b, 514);
    }

    #[test]
    fn test_rendezvous_distribution2() {
        let mut r = Rendezvous::with_me("ServerA".to_owned());
        r.add_node("ServerB".to_owned());
        r.add_node("ServerC".to_owned());
        r.add_node("ServerD".to_owned());

        let mut counts = HashMap::<String, u32>::new();
        for i in 0..1000 {
            let target = r.node_for_item(&format!("node{i}")).unwrap();
            *counts.entry(target.clone()).or_default() += 1;
        }

        assert_eq!(*counts.get("ServerA").unwrap(), 225);
        assert_eq!(*counts.get("ServerB").unwrap(), 252);
        assert_eq!(*counts.get("ServerC").unwrap(), 275);
        assert_eq!(*counts.get("ServerD").unwrap(), 248);
    }
}
