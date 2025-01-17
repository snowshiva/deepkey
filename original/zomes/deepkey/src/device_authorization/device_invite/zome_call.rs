use hdk::prelude::*;
use crate::device_authorization::device_invite::entry::DeviceInvite;
use crate::device_authorization::device_invite_acceptance::entry::DeviceInviteAcceptance;
use crate::device_authorization::inbox::DEVICE_INVITE_LINK_TAG_BYTES;
use crate::device_authorization::device_invite::local_keyset_parent;

#[hdk_extern]
fn invite_agents(invitees: Vec<AgentPubKey>) -> ExternResult<Vec<DeviceInviteAcceptance>> {
    let mut acceptances: Vec<DeviceInviteAcceptance> = Vec::new();
    for invitee in invitees.iter() {
        let (keyset_root, parent) = local_keyset_parent()?;
        let invite = DeviceInvite::new(
            keyset_root.clone(),
            parent,
            invitee.clone(),
        );
        let invite_header = create_entry(invite.clone())?;
        create_link(
            invitee.clone().into(),
            hash_entry(invite)?,
            LinkTag(DEVICE_INVITE_LINK_TAG_BYTES.iter().chain(invite_header.get_raw_39().iter()).cloned().collect::<Vec<u8>>()),
        )?;
        acceptances.push(DeviceInviteAcceptance::new(
            keyset_root.clone(),
            invite_header,
        ));
    }

    Ok(acceptances)
}