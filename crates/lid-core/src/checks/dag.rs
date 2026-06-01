//! Arrow dependency DAG validation.
//!
//! Implements §14 of the audit-checklist. Builds a directed graph from
//! the `blocks` relations in `index.yaml` (A → B means "A blocks B")
//! and looks for cycles via Tarjan's SCC algorithm. Any strongly
//! connected component of size >1 is a multi-segment cycle; an SCC of
//! size 1 carrying a self-edge is a segment that blocks itself.
//!
//! Cycles violate the methodology's "arrow of intent" — segments must
//! form a DAG so that "block / blockedBy" describes a real ordering.
//! Findings ship at Error severity because no other check can recover
//! sensible behaviour while a cycle is present.

use std::collections::BTreeMap;

use petgraph::algo::tarjan_scc;
use petgraph::graph::DiGraph;

use crate::LidRepo;
use crate::model::SegmentId;

use super::{Category, Check, CheckId, Finding, Location, Severity};

pub struct DagCheck;

impl Check for DagCheck {
    fn id(&self) -> CheckId {
        CheckId::Dag
    }

    fn run(&self, repo: &LidRepo) -> Vec<Finding> {
        let mut graph: DiGraph<&SegmentId, ()> = DiGraph::new();
        let mut node_map: BTreeMap<&SegmentId, _> = BTreeMap::new();
        for seg_id in repo.index.arrows.keys() {
            let idx = graph.add_node(seg_id);
            node_map.insert(seg_id, idx);
        }
        for (seg_id, seg) in &repo.index.arrows {
            let Some(&from) = node_map.get(seg_id) else {
                continue;
            };
            for target in &seg.blocks {
                if let Some(&to) = node_map.get(target) {
                    graph.add_edge(from, to, ());
                }
            }
        }

        let mut findings = Vec::new();
        let index_yaml = repo.root.join("docs").join("arrows").join("index.yaml");

        for scc in tarjan_scc(&graph) {
            if scc.len() == 1 {
                // A solo SCC carrying a self-edge is a self-loop.
                let node = scc[0];
                if graph.edges_connecting(node, node).next().is_some() {
                    findings.push(self_loop_finding(graph[node], &index_yaml));
                }
                continue;
            }
            // Multi-node SCCs are cycles. Sort members for stable output.
            let mut members: Vec<&SegmentId> = scc.iter().map(|&idx| graph[idx]).collect();
            members.sort();
            findings.push(cycle_finding(&members, &index_yaml));
        }
        findings
    }
}

fn self_loop_finding(seg: &SegmentId, index_yaml: &std::path::Path) -> Finding {
    Finding {
        check: CheckId::Dag,
        severity: Severity::Error,
        category: Category::Dag,
        message: format!("segment `{seg}` blocks itself — `blocks` must form a DAG"),
        location: Some(Location {
            path: index_yaml.to_path_buf(),
            line: None,
        }),
        spec: None,
        remediation: Some(format!("remove `{seg}` from its own `blocks` list")),
    }
}

fn cycle_finding(members: &[&SegmentId], index_yaml: &std::path::Path) -> Finding {
    let chain = members
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(" → ");
    Finding {
        check: CheckId::Dag,
        severity: Severity::Error,
        category: Category::Dag,
        message: format!(
            "arrow dependency cycle: {} segments form a cycle ({chain})",
            members.len()
        ),
        location: Some(Location {
            path: index_yaml.to_path_buf(),
            line: None,
        }),
        spec: None,
        remediation: Some(
            "remove one of the `blocks` edges among the listed segments to break the cycle"
                .to_owned(),
        ),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::model::{ArrowIndex, Segment, Status, Unmapped};

    fn seg_id(s: &str) -> SegmentId {
        SegmentId::parse(s).unwrap()
    }

    fn seg(blocks: &[&str]) -> Segment {
        Segment {
            status: Status::Mapped,
            sampled: None,
            audited: None,
            audited_sha: None,
            blocks: blocks.iter().map(|s| seg_id(s)).collect(),
            blocked_by: vec![],
            detail: PathBuf::from("stub.md"),
            next: None,
            drift: None,
            merged_into: None,
            children: vec![],
            parent: None,
        }
    }

    fn repo_with(arrows: &[(&str, &[&str])]) -> LidRepo {
        let mut map = BTreeMap::new();
        for (name, blocks) in arrows {
            map.insert(seg_id(name), seg(blocks));
        }
        LidRepo {
            root: PathBuf::from("/fake/root"),
            index: ArrowIndex {
                schema_version: 1,
                last_updated: None,
                taxonomy: BTreeMap::new(),
                arrows: map,
                unmapped: Unmapped::default(),
            },
            specs: vec![],
            llds: vec![],
            arrow_docs: vec![],
            citations: vec![],
            decision_docs: vec![],
        }
    }

    #[test]
    fn empty_index_yields_no_findings() {
        let repo = repo_with(&[]);
        let f = DagCheck.run(&repo);
        assert!(f.is_empty());
    }

    #[test]
    fn linear_chain_is_acyclic() {
        // a → b → c
        let repo = repo_with(&[("a", &["b"]), ("b", &["c"]), ("c", &[])]);
        let f = DagCheck.run(&repo);
        assert!(f.is_empty(), "got {f:?}");
    }

    #[test]
    fn diamond_is_acyclic() {
        // a → b, a → c, b → d, c → d
        let repo = repo_with(&[("a", &["b", "c"]), ("b", &["d"]), ("c", &["d"]), ("d", &[])]);
        let f = DagCheck.run(&repo);
        assert!(f.is_empty(), "got {f:?}");
    }

    #[test]
    fn two_node_cycle_is_flagged() {
        let repo = repo_with(&[("a", &["b"]), ("b", &["a"])]);
        let f = DagCheck.run(&repo);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Error);
        assert_eq!(f[0].category, Category::Dag);
        assert!(f[0].message.contains("cycle"));
        assert!(f[0].message.contains('a'));
        assert!(f[0].message.contains('b'));
    }

    #[test]
    fn three_node_cycle_is_flagged() {
        let repo = repo_with(&[("a", &["b"]), ("b", &["c"]), ("c", &["a"])]);
        let f = DagCheck.run(&repo);
        assert_eq!(f.len(), 1);
        assert!(f[0].message.contains("3 segments"));
    }

    #[test]
    fn self_loop_is_flagged() {
        let repo = repo_with(&[("solo", &["solo"])]);
        let f = DagCheck.run(&repo);
        assert_eq!(f.len(), 1);
        assert!(
            f[0].message.contains("blocks itself"),
            "got {}",
            f[0].message
        );
    }

    #[test]
    fn two_independent_cycles_each_get_a_finding() {
        let repo = repo_with(&[("a", &["b"]), ("b", &["a"]), ("x", &["y"]), ("y", &["x"])]);
        let f = DagCheck.run(&repo);
        assert_eq!(f.len(), 2);
    }

    #[test]
    fn unknown_block_targets_are_ignored_for_cycle_purposes() {
        // `a` blocks an unknown segment — that's a schema-check concern,
        // not a DAG concern. DagCheck just doesn't draw the edge.
        let repo = repo_with(&[("a", &["missing"])]);
        let f = DagCheck.run(&repo);
        assert!(f.is_empty(), "got {f:?}");
    }

    #[test]
    fn check_id_is_dag() {
        assert_eq!(DagCheck.id(), CheckId::Dag);
    }
}
