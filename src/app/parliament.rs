use super::*;
use crate::api::{PlCluster, PlIssue};

impl App {
    pub async fn pl_fetch_data(&mut self) {
        match self.client.pl_get_parliament().await {
            Ok(parliament) => {
                self.parliament.cont3xt_url = parliament.settings.general.cont3xt_url.clone();
                self.parliament.wise_url = parliament.settings.general.wise_url.clone();
                self.parliament.settings_general = parliament.settings.general;
                self.parliament.groups = parliament.groups;
                self.pl_rebuild_cluster_list();
                self.pl_rebuild_settings_items();
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

    /// Build the flat list of settings items for group/cluster navigation
    pub(crate) fn pl_rebuild_settings_items(&mut self) {
        self.parliament.settings_items.clear();
        for (gi, group) in self.parliament.groups.iter().enumerate() {
            self.parliament.settings_items.push((gi, None));
            for (ci, _) in group.clusters.iter().enumerate() {
                self.parliament.settings_items.push((gi, Some(ci)));
            }
        }
    }

    /// Open group editor for existing group
    pub(crate) fn pl_open_group_editor(&mut self, group_idx: usize) {
        if let Some(group) = self.parliament.groups.get(group_idx) {
            self.parliament.group_editor_idx = group_idx;
            self.parliament.group_editor_is_new = false;
            self.parliament.group_editor_title = group.title.clone();
            self.parliament.group_editor_title_cursor = group.title.len();
            self.parliament.group_editor_desc = group.description.clone();
            self.parliament.group_editor_desc_cursor = group.description.len();
            self.parliament.group_editor_field = PlGroupEditorField::Title;
            self.parliament.settings_level = PlSettingsLevel::GroupEditor;
        }
    }

    /// Open group editor for new group
    pub(crate) fn pl_open_new_group_editor(&mut self) {
        self.parliament.group_editor_idx = 0;
        self.parliament.group_editor_is_new = true;
        self.parliament.group_editor_title.clear();
        self.parliament.group_editor_title_cursor = 0;
        self.parliament.group_editor_desc.clear();
        self.parliament.group_editor_desc_cursor = 0;
        self.parliament.group_editor_field = PlGroupEditorField::Title;
        self.parliament.settings_level = PlSettingsLevel::GroupEditor;
    }

    /// Open cluster editor for existing cluster
    pub(crate) fn pl_open_cluster_editor(&mut self, group_idx: usize, cluster_idx: usize) {
        if let Some(cluster) = self.parliament.groups.get(group_idx)
            .and_then(|g| g.clusters.get(cluster_idx))
        {
            self.parliament.cluster_editor_group_idx = group_idx;
            self.parliament.cluster_editor_cluster_idx = cluster_idx;
            self.parliament.cluster_editor_is_new = false;
            self.parliament.cluster_editor_field = PlClusterEditorField::Title;
            self.parliament.cluster_editor_title = cluster.title.clone();
            self.parliament.cluster_editor_title_cursor = cluster.title.len();
            self.parliament.cluster_editor_url = cluster.url.clone();
            self.parliament.cluster_editor_url_cursor = cluster.url.len();
            self.parliament.cluster_editor_local_url = cluster.local_url.clone();
            self.parliament.cluster_editor_local_url_cursor = cluster.local_url.len();
            self.parliament.cluster_editor_desc = cluster.description.clone();
            self.parliament.cluster_editor_desc_cursor = cluster.description.len();
            self.parliament.cluster_editor_type = cluster.cluster_type.clone();
            self.parliament.cluster_editor_hide_delta_bps = cluster.hide_delta_bps;
            self.parliament.cluster_editor_hide_delta_tdps = cluster.hide_delta_tdps;
            self.parliament.cluster_editor_hide_monitoring = cluster.hide_monitoring;
            self.parliament.cluster_editor_hide_arkime_nodes = cluster.hide_arkime_nodes;
            self.parliament.cluster_editor_hide_data_nodes = cluster.hide_data_nodes;
            self.parliament.cluster_editor_hide_total_nodes = cluster.hide_total_nodes;
            self.parliament.settings_level = PlSettingsLevel::ClusterEditor;
        }
    }

    /// Open cluster editor for new cluster in given group
    pub(crate) fn pl_open_new_cluster_editor(&mut self, group_idx: usize) {
        self.parliament.cluster_editor_group_idx = group_idx;
        self.parliament.cluster_editor_cluster_idx = 0;
        self.parliament.cluster_editor_is_new = true;
        self.parliament.cluster_editor_field = PlClusterEditorField::Title;
        self.parliament.cluster_editor_title.clear();
        self.parliament.cluster_editor_title_cursor = 0;
        self.parliament.cluster_editor_url.clear();
        self.parliament.cluster_editor_url_cursor = 0;
        self.parliament.cluster_editor_local_url.clear();
        self.parliament.cluster_editor_local_url_cursor = 0;
        self.parliament.cluster_editor_desc.clear();
        self.parliament.cluster_editor_desc_cursor = 0;
        self.parliament.cluster_editor_type.clear();
        self.parliament.cluster_editor_hide_delta_bps = false;
        self.parliament.cluster_editor_hide_delta_tdps = false;
        self.parliament.cluster_editor_hide_monitoring = false;
        self.parliament.cluster_editor_hide_arkime_nodes = false;
        self.parliament.cluster_editor_hide_data_nodes = false;
        self.parliament.cluster_editor_hide_total_nodes = false;
        self.parliament.settings_level = PlSettingsLevel::ClusterEditor;
    }

    /// Save group editor (create or update)
    pub(crate) async fn pl_save_group(&mut self) {
        let title = self.parliament.group_editor_title.trim().to_string();
        if title.is_empty() {
            self.status_msg = "Group title cannot be empty".to_string();
            return;
        }
        let desc = self.parliament.group_editor_desc.trim().to_string();
        let result = if self.parliament.group_editor_is_new {
            self.client.pl_create_group(&title, &desc).await
        } else {
            let group_id = self.parliament.groups.get(self.parliament.group_editor_idx)
                .map(|g| g.id.clone()).unwrap_or_default();
            self.client.pl_update_group(&group_id, &title, &desc).await
        };
        match result {
            Ok(()) => {
                self.status_msg = if self.parliament.group_editor_is_new {
                    "Group created".to_string()
                } else {
                    "Group updated".to_string()
                };
                self.parliament.settings_level = PlSettingsLevel::GroupList;
                self.pl_fetch_data().await;
            }
            Err(e) => self.status_msg = format!("Error: {e}"),
        }
    }

    /// Save cluster editor (create or update)
    pub(crate) async fn pl_save_cluster(&mut self) {
        let title = self.parliament.cluster_editor_title.trim().to_string();
        if title.is_empty() {
            self.status_msg = "Cluster title cannot be empty".to_string();
            return;
        }
        let url = self.parliament.cluster_editor_url.trim().to_string();
        if url.is_empty() || !url.starts_with("http") {
            self.status_msg = "Cluster URL must start with http".to_string();
            return;
        }
        let cluster_json = serde_json::json!({
            "title": title,
            "url": url,
            "description": self.parliament.cluster_editor_desc.trim(),
            "localUrl": self.parliament.cluster_editor_local_url.trim(),
            "type": self.parliament.cluster_editor_type.trim(),
            "hideDeltaBPS": self.parliament.cluster_editor_hide_delta_bps,
            "hideDeltaTDPS": self.parliament.cluster_editor_hide_delta_tdps,
            "hideMonitoring": self.parliament.cluster_editor_hide_monitoring,
            "hideArkimeNodes": self.parliament.cluster_editor_hide_arkime_nodes,
            "hideDataNodes": self.parliament.cluster_editor_hide_data_nodes,
            "hideTotalNodes": self.parliament.cluster_editor_hide_total_nodes,
        });
        let gi = self.parliament.cluster_editor_group_idx;
        let group_id = self.parliament.groups.get(gi)
            .map(|g| g.id.clone()).unwrap_or_default();
        let result = if self.parliament.cluster_editor_is_new {
            self.client.pl_create_cluster(&group_id, &cluster_json).await
        } else {
            let cluster_id = self.parliament.groups.get(gi)
                .and_then(|g| g.clusters.get(self.parliament.cluster_editor_cluster_idx))
                .and_then(|c| c.id.clone()).unwrap_or_default();
            self.client.pl_update_cluster(&group_id, &cluster_id, &cluster_json).await
        };
        match result {
            Ok(()) => {
                self.status_msg = if self.parliament.cluster_editor_is_new {
                    "Cluster created".to_string()
                } else {
                    "Cluster updated".to_string()
                };
                self.parliament.settings_level = PlSettingsLevel::GroupList;
                self.pl_fetch_data().await;
            }
            Err(e) => self.status_msg = format!("Error: {e}"),
        }
    }

    /// Delete selected group or cluster
    pub(crate) async fn pl_delete_selected(&mut self) {
        let sel = self.parliament.settings_selected;
        if let Some(&(gi, ci_opt)) = self.parliament.settings_items.get(sel) {
            match ci_opt {
                None => {
                    // Delete group
                    let group_id = self.parliament.groups.get(gi)
                        .map(|g| g.id.clone()).unwrap_or_default();
                    match self.client.pl_delete_group(&group_id).await {
                        Ok(()) => {
                            self.status_msg = "Group deleted".to_string();
                            self.pl_fetch_data().await;
                            if self.parliament.settings_selected >= self.parliament.settings_items.len() && !self.parliament.settings_items.is_empty() {
                                self.parliament.settings_selected = self.parliament.settings_items.len() - 1;
                            }
                            self.parliament.settings_table_state.select(Some(self.parliament.settings_selected));
                        }
                        Err(e) => self.status_msg = format!("Error deleting group: {e}"),
                    }
                }
                Some(ci) => {
                    // Delete cluster
                    let group_id = self.parliament.groups.get(gi)
                        .map(|g| g.id.clone()).unwrap_or_default();
                    let cluster_id = self.parliament.groups.get(gi)
                        .and_then(|g| g.clusters.get(ci))
                        .and_then(|c| c.id.clone()).unwrap_or_default();
                    match self.client.pl_delete_cluster(&group_id, &cluster_id).await {
                        Ok(()) => {
                            self.status_msg = "Cluster deleted".to_string();
                            self.pl_fetch_data().await;
                            if self.parliament.settings_selected >= self.parliament.settings_items.len() && !self.parliament.settings_items.is_empty() {
                                self.parliament.settings_selected = self.parliament.settings_items.len() - 1;
                            }
                            self.parliament.settings_table_state.select(Some(self.parliament.settings_selected));
                        }
                        Err(e) => self.status_msg = format!("Error deleting cluster: {e}"),
                    }
                }
            }
        }
    }

    /// Save general settings to server
    pub(crate) async fn pl_save_general_settings(&mut self) {
        let s = &self.parliament.settings_general;
        let settings = serde_json::json!({
            "outOfDate": s.out_of_date.unwrap_or(30),
            "esQueryTimeout": s.es_query_timeout.unwrap_or(5),
            "noPackets": s.no_packets.unwrap_or(0),
            "noPacketsLength": s.no_packets_length.unwrap_or(10),
            "lowDiskSpace": s.low_disk_space.unwrap_or(4.0),
            "lowDiskSpaceType": s.low_disk_space_type.clone().unwrap_or_else(|| "percentage".to_string()),
            "lowDiskSpaceES": s.low_disk_space_es.unwrap_or(15.0),
            "lowDiskSpaceESType": s.low_disk_space_es_type.clone().unwrap_or_else(|| "percentage".to_string()),
            "removeIssuesAfter": s.remove_issues_after.unwrap_or(60),
            "removeAcknowledgedAfter": s.remove_acknowledged_after.unwrap_or(15),
            "wiseUrl": s.wise_url,
            "cont3xtUrl": s.cont3xt_url,
        });
        match self.client.pl_update_settings(&settings).await {
            Ok(()) => self.status_msg = "Settings saved".to_string(),
            Err(e) => self.status_msg = format!("Error saving settings: {e}"),
        }
    }

    /// Get the display value for a general settings field
    pub(crate) fn pl_general_field_value(&self, field: &PlGeneralField) -> String {
        use PlGeneralField::*;
        let s = &self.parliament.settings_general;
        match field {
            OutOfDate => s.out_of_date.map(|v| v.to_string()).unwrap_or_else(|| "30".into()),
            EsQueryTimeout => s.es_query_timeout.map(|v| v.to_string()).unwrap_or_else(|| "5".into()),
            NoPackets => s.no_packets.map(|v| v.to_string()).unwrap_or_else(|| "0".into()),
            NoPacketsLength => s.no_packets_length.map(|v| v.to_string()).unwrap_or_else(|| "10".into()),
            LowDiskSpace => s.low_disk_space.map(|v| format!("{v}")).unwrap_or_else(|| "4".into()),
            LowDiskSpaceType => s.low_disk_space_type.clone().unwrap_or_else(|| "percentage".into()),
            LowDiskSpaceES => s.low_disk_space_es.map(|v| format!("{v}")).unwrap_or_else(|| "15".into()),
            LowDiskSpaceESType => s.low_disk_space_es_type.clone().unwrap_or_else(|| "percentage".into()),
            RemoveIssuesAfter => s.remove_issues_after.map(|v| v.to_string()).unwrap_or_else(|| "60".into()),
            RemoveAcknowledgedAfter => s.remove_acknowledged_after.map(|v| v.to_string()).unwrap_or_else(|| "15".into()),
            WiseUrl => s.wise_url.clone(),
            Cont3xtUrl => s.cont3xt_url.clone(),
        }
    }

    /// Apply an edited value to a general settings field
    pub(crate) fn pl_set_general_field(&mut self, field: &PlGeneralField, value: &str) {
        use PlGeneralField::*;
        let s = &mut self.parliament.settings_general;
        match field {
            OutOfDate => s.out_of_date = value.parse().ok(),
            EsQueryTimeout => s.es_query_timeout = value.parse().ok(),
            NoPackets => s.no_packets = value.parse().ok(),
            NoPacketsLength => s.no_packets_length = value.parse().ok(),
            LowDiskSpace => s.low_disk_space = value.parse().ok(),
            LowDiskSpaceType => s.low_disk_space_type = Some(value.to_string()),
            LowDiskSpaceES => s.low_disk_space_es = value.parse().ok(),
            LowDiskSpaceESType => s.low_disk_space_es_type = Some(value.to_string()),
            RemoveIssuesAfter => s.remove_issues_after = value.parse().ok(),
            RemoveAcknowledgedAfter => s.remove_acknowledged_after = value.parse().ok(),
            WiseUrl => s.wise_url = value.to_string(),
            Cont3xtUrl => s.cont3xt_url = value.to_string(),
        }
    }
}
