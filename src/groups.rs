use anyhow::Result;
use wacore_binary::builder::NodeBuilder;
use wacore_binary::jid::Jid;
use wacore_binary::node::NodeContent;
use whatsapp_rust::Client;

/// Group metadata including name and participants
#[derive(Debug, Clone)]
pub struct GroupMetadata {
    #[allow(dead_code)]
    pub jid: Jid,
    pub subject: String,
    pub participant_count: usize,
}

/// Extension trait to add group management functionality to the WhatsApp Client
#[allow(async_fn_in_trait)]
pub trait GroupManagement {
    /// Query group metadata including name (subject) and participant count
    ///
    /// # Arguments
    /// * `group_jid` - The JID of the group (format: "1234567890-1234567890@g.us")
    ///
    /// # Returns
    /// Result containing GroupMetadata with the group name and participant count
    async fn query_group_metadata(&self, group_jid: &Jid) -> Result<GroupMetadata>;
    /// Add participants to a WhatsApp group
    ///
    /// # Arguments
    /// * `group_jid` - The JID of the group (format: "1234567890-1234567890@g.us")
    /// * `participant_jids` - List of participant JIDs to add (format: "1234567890@s.whatsapp.net")
    ///
    /// # Returns
    /// Result containing a vector of tuples with (participant_jid, success: bool, error_code: Option<u64>)
    ///
    /// # Example
    /// ```no_run
    /// use wacore_binary::jid::Jid;
    ///
    /// let group_jid: Jid = "1234567890-1234567890@g.us".parse()?;
    /// let participants = vec![
    ///     "1234567890@s.whatsapp.net".parse()?,
    ///     "0987654321@s.whatsapp.net".parse()?,
    /// ];
    ///
    /// let results = client.add_group_participants(&group_jid, &participants).await?;
    /// for (jid, success, error_code) in results {
    ///     if success {
    ///         println!("Successfully added {}", jid);
    ///     } else {
    ///         println!("Failed to add {} with error code {:?}", jid, error_code);
    ///     }
    /// }
    /// ```
    async fn add_group_participants(
        &self,
        group_jid: &Jid,
        participant_jids: &[Jid],
    ) -> Result<Vec<(Jid, bool, Option<u64>)>>;

    /// Remove participants from a WhatsApp group
    ///
    /// # Arguments
    /// * `group_jid` - The JID of the group (format: "1234567890-1234567890@g.us")
    /// * `participant_jids` - List of participant JIDs to remove (format: "1234567890@s.whatsapp.net")
    ///
    /// # Returns
    /// Result containing a vector of tuples with (participant_jid, success: bool, error_code: Option<u64>)
    #[allow(dead_code)]
    async fn remove_group_participants(
        &self,
        group_jid: &Jid,
        participant_jids: &[Jid],
    ) -> Result<Vec<(Jid, bool, Option<u64>)>>;

    /// Get the invite link for a WhatsApp group
    ///
    /// # Arguments
    /// * `group_jid` - The JID of the group (format: "1234567890-1234567890@g.us")
    ///
    /// # Returns
    /// Result containing the invite link (format: "https://chat.whatsapp.com/XXXXXX")
    #[allow(dead_code)]
    async fn get_group_invite_link(&self, group_jid: &Jid) -> Result<String>;
}

impl GroupManagement for Client {
    async fn query_group_metadata(&self, group_jid: &Jid) -> Result<GroupMetadata> {
        let query_node = NodeBuilder::new("query")
            .attr("request", "interactive")
            .build();

        let iq = whatsapp_rust::request::InfoQuery {
            namespace: "w:g2",
            query_type: whatsapp_rust::request::InfoQueryType::Get,
            to: group_jid.clone(),
            content: Some(NodeContent::Nodes(vec![query_node])),
            id: None,
            target: None,
            timeout: None,
        };

        let resp_node = self.send_iq(iq).await?;

        let group_node = resp_node
            .get_optional_child("group")
            .ok_or_else(|| anyhow::anyhow!("<group> not found in group info response"))?;

        let mut parser = wacore_binary::attrs::AttrParser::new(group_node);
        let subject = parser
            .optional_string("subject")
            .unwrap_or("Unknown Group")
            .to_string();

        let participant_count = group_node.get_children_by_tag("participant").len();

        Ok(GroupMetadata {
            jid: group_jid.clone(),
            subject,
            participant_count,
        })
    }

    async fn add_group_participants(
        &self,
        group_jid: &Jid,
        participant_jids: &[Jid],
    ) -> Result<Vec<(Jid, bool, Option<u64>)>> {
        if participant_jids.is_empty() {
            return Ok(vec![]);
        }

        // Build participant nodes
        let participant_nodes: Vec<_> = participant_jids
            .iter()
            .map(|jid| {
                NodeBuilder::new("participant")
                    .attr("jid", jid.to_string())
                    .build()
            })
            .collect();

        // Build add node with participants
        let add_node = NodeBuilder::new("add").children(participant_nodes).build();

        // Build the IQ query
        let iq = whatsapp_rust::request::InfoQuery {
            namespace: "w:g2",
            query_type: whatsapp_rust::request::InfoQueryType::Set,
            to: group_jid.clone(),
            content: Some(NodeContent::Nodes(vec![add_node])),
            id: None,
            target: None,
            timeout: None,
        };

        // Send the IQ and get response
        let resp_node = self.send_iq(iq).await?;

        // Parse the response to check for errors or success
        let mut results = Vec::new();

        if let Some(add_response) = resp_node.get_optional_child("add") {
            for participant_node in add_response.get_children_by_tag("participant") {
                let mut parser = wacore_binary::attrs::AttrParser::new(participant_node);
                let jid = parser.jid("jid");
                let error_code = parser.optional_u64("error");

                if let Some(code) = error_code {
                    log::warn!("Failed to add participant {}: error code {}", jid, code);
                    results.push((jid, false, Some(code)));
                } else {
                    log::info!("Successfully added participant: {}", jid);
                    results.push((jid, true, None));
                }
            }
        }

        Ok(results)
    }

    async fn remove_group_participants(
        &self,
        group_jid: &Jid,
        participant_jids: &[Jid],
    ) -> Result<Vec<(Jid, bool, Option<u64>)>> {
        if participant_jids.is_empty() {
            return Ok(vec![]);
        }

        // Build participant nodes
        let participant_nodes: Vec<_> = participant_jids
            .iter()
            .map(|jid| {
                NodeBuilder::new("participant")
                    .attr("jid", jid.to_string())
                    .build()
            })
            .collect();

        // Build remove node with participants
        let remove_node = NodeBuilder::new("remove")
            .children(participant_nodes)
            .build();

        // Build the IQ query
        let iq = whatsapp_rust::request::InfoQuery {
            namespace: "w:g2",
            query_type: whatsapp_rust::request::InfoQueryType::Set,
            to: group_jid.clone(),
            content: Some(NodeContent::Nodes(vec![remove_node])),
            id: None,
            target: None,
            timeout: None,
        };

        // Send the IQ and get response
        let resp_node = self.send_iq(iq).await?;

        // Parse the response to check for errors or success
        let mut results = Vec::new();

        if let Some(remove_response) = resp_node.get_optional_child("remove") {
            for participant_node in remove_response.get_children_by_tag("participant") {
                let mut parser = wacore_binary::attrs::AttrParser::new(participant_node);
                let jid = parser.jid("jid");
                let error_code = parser.optional_u64("error");

                if let Some(code) = error_code {
                    log::warn!("Failed to remove participant {}: error code {}", jid, code);
                    results.push((jid, false, Some(code)));
                } else {
                    log::info!("Successfully removed participant: {}", jid);
                    results.push((jid, true, None));
                }
            }
        }

        Ok(results)
    }

    async fn get_group_invite_link(&self, group_jid: &Jid) -> Result<String> {
        let invite_node = NodeBuilder::new("invite").build();

        let iq = whatsapp_rust::request::InfoQuery {
            namespace: "w:g2",
            query_type: whatsapp_rust::request::InfoQueryType::Get,
            to: group_jid.clone(),
            content: Some(NodeContent::Nodes(vec![invite_node])),
            id: None,
            target: None,
            timeout: None,
        };

        let resp_node = self.send_iq(iq).await?;

        let invite_response = resp_node
            .get_optional_child("invite")
            .ok_or_else(|| anyhow::anyhow!("<invite> not found in response"))?;

        let mut parser = wacore_binary::attrs::AttrParser::new(invite_response);
        let invite_code = parser
            .optional_string("code")
            .ok_or_else(|| anyhow::anyhow!("Invite code not found"))?;

        Ok(format!("https://chat.whatsapp.com/{}", invite_code))
    }
}
