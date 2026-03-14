use super::*;
use crate::api::{PlCluster, PlIssue};

impl App {
    pub async fn pl_fetch_data(&mut self) {
        match self.client.pl_get_parliament().await {
            Ok(parliament) => {
                self.parliament.cont3xt_url = parliament.settings.general.cont3xt_url.clone();
                self.parliament.wise_url = parliament.settings.general.wise_url.clone();
                self.parliament.groups = parliament.groups;
                self.pl_rebuild_cluster_list();
                self.status_msg = format!("{} groups loaded", self.parliament.groups.len());
            }
            Err(e) => self.status_msg = format!("Error fetching parliament: {e}"),
        }
        match self.client.pl_get_stats().await {
            Ok(stats) => self.parliament.stats = stats,
            Err(e) => self.status_msg = format!("Error fetching stats: {e}"),
        }
        match self.client.pl_get_issues_map().await {
            Ok(issues) => self.parliament.issues_map = issues,
            Err(e) => self.status_msg = format!("Error fetching issues: {e}"),
        }
        self.parliament.last_refresh = std::time::Instant::now();
    }

    pub async fn pl_fetch_issues(&mut self) {
        match self.client.pl_get_issues().await {
            Ok(issues) => {
                let count = issues.len();
                self.parliament.issues = issues;
                self.pl_sort_issues();
                self.status_msg = format!("{} issues", count);
            }
            Err(e) => self.status_msg = format!("Error fetching issues: {e}"),
        }
    }

    pub(crate) fn pl_rebuild_cluster_list(&mut self) {
        self.parliament.cluster_list.clear();
        for (gi, group) in self.parliament.groups.iter().enumerate() {
            for (ci, _cluster) in group.clusters.iter().enumerate() {
                self.parliament.cluster_list.push((gi, ci));
            }
        }
    }

    pub(crate) fn pl_sort_issues(&mut self) {
        let sort = self.parliament.issues_sort;
        let desc = self.parliament.issues_sort_desc;
        self.parliament.issues.sort_by(|a, b| {
            let cmp = match sort {
                PlIssueSort::Cluster => a.cluster.to_lowercase().cmp(&b.cluster.to_lowercase()),
                PlIssueSort::Title => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
                PlIssueSort::Severity => a.severity.cmp(&b.severity),
                PlIssueSort::FirstNoticed => a.first_noticed.cmp(&b.first_noticed),
                PlIssueSort::LastNoticed => a.last_noticed.cmp(&b.last_noticed),
            };
            if desc { cmp.reverse() } else { cmp }
        });
    }

    /// Get the currently selected cluster on the dashboard
    pub(crate) fn pl_selected_cluster_ref(&self) -> Option<&PlCluster> {
        if self.parliament.cluster_list.is_empty() {
            return None;
        }
        let nav_idx = self.pl_dashboard_nav_index();
        if nav_idx < self.parliament.cluster_list.len() {
            let (gi, ci) = self.parliament.cluster_list[nav_idx];
            self.parliament.groups.get(gi).and_then(|g| g.clusters.get(ci))
        } else {
            None
        }
    }

    /// Get flat index from current group/cluster selection
    pub(crate) fn pl_dashboard_nav_index(&self) -> usize {
        self.parliament.cluster_list.iter().position(|&(gi, ci)| gi == self.parliament.selected_group && ci == self.parliament.selected_cluster).unwrap_or(0)
    }

    /// Get filtered issues list
    pub(crate) fn pl_filtered_issues(&self) -> Vec<&PlIssue> {
        let filter = self.parliament.issues_filter.to_lowercase();
        self.parliament.issues.iter().filter(|issue| {
            if filter.is_empty() {
                return true;
            }
            issue.cluster.to_lowercase().contains(&filter)
                || issue.title.to_lowercase().contains(&filter)
                || issue.message.to_lowercase().contains(&filter)
                || issue.node.to_lowercase().contains(&filter)
                || issue.severity.to_lowercase().contains(&filter)
        }).collect()
    }
}
