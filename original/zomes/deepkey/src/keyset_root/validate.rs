use hdk::prelude::*;
use crate::keyset_root::entry::KeysetRoot;
use crate::keyset_root::entry::KEYSET_ROOT_CHAIN_INDEX;
use crate::keyset_root::error::Error;

impl KeysetRoot {
    pub fn verify_signature(&self) -> ExternResult<bool> {
        verify_signature_raw(
            self.as_root_pub_key_ref().to_owned(),
            self.as_fda_pubkey_signed_by_root_key_ref().to_owned(),
            self.as_first_deepkey_agent_ref().get_raw_32().to_vec()
        )
    }
}

impl TryFrom<&Element> for KeysetRoot {
    type Error = Error;
    fn try_from(element: &Element) -> Result<Self, Self::Error> {
        match element.header() {
            // Only creates are allowed for a KeysetRoot.
            Header::Create(_) => {
                Ok(match element.entry() {
                    ElementEntry::Present(serialized_keyset_root) => match KeysetRoot::try_from(serialized_keyset_root) {
                        Ok(keyset_root) => keyset_root,
                        Err(e) => return Err(Error::Wasm(e)),
                    },
                    _ => return Err(Error::EntryMissing),
                })
            },
            _ => Err(Error::WrongHeader),
        }

    }
}

fn _validate_create_header(create_header: &Create) -> ExternResult<ValidateCallbackResult> {
    // Header needs to be in the correct position in the chain.
    if create_header.header_seq != KEYSET_ROOT_CHAIN_INDEX {
        Error::Position(create_header.header_seq, KEYSET_ROOT_CHAIN_INDEX).into()
    }
    else {
        Ok(ValidateCallbackResult::Valid)
    }
}

fn _validate_create_authorization(create_header: &Create, proposed_keyset_root: &KeysetRoot) -> ExternResult<ValidateCallbackResult> {
    // The author must be the FDA.
    if *proposed_keyset_root.as_first_deepkey_agent_ref() != create_header.author {
        return Error::FdaAuthor.into();
    }
    // The signature must be correct.
    else if !proposed_keyset_root.verify_signature()? {
        return Error::FdaSignature.into();
    }
    else {
        Ok(ValidateCallbackResult::Valid)
    }
}

#[hdk_extern]
/// Create only.
fn validate_create_entry_keyset_root(validate_data: ValidateData) -> ExternResult<ValidateCallbackResult> {
    let proposed_keyset_root = match KeysetRoot::try_from(&validate_data.element) {
        Ok(keyset_root) => keyset_root,
        Err(e) => return Ok(ValidateCallbackResult::Invalid(e.to_string())),
    };

    match validate_data.element.header() {
        Header::Create(create_header) => {
            match _validate_create_header(&create_header) {
                Ok(ValidateCallbackResult::Valid) => {},
                validate_callback_result => return validate_callback_result,
            }

            match _validate_create_authorization(&create_header, &proposed_keyset_root) {
                Ok(ValidateCallbackResult::Valid) => {},
                validate_callback_result => return validate_callback_result,
            }

            Ok(ValidateCallbackResult::Valid)
        },
        Header::Update(_) => Error::UpdateAttempted.into(),
        Header::Delete(_) => Error::DeleteAttempted.into(),
        _ => Error::WrongHeader.into(),
    }
}

#[hdk_extern]
/// Updates are not allowed for KeysetRoot.
fn validate_update_entry_keyset_root(_: ValidateData) -> ExternResult<ValidateCallbackResult> {
    Error::UpdateAttempted.into()
}

#[hdk_extern]
/// Deletes are not allowed for KeysetRoot.
fn validate_delete_entry_keyset_root(_: ValidateData) -> ExternResult<ValidateCallbackResult> {
    Error::DeleteAttempted.into()
}

#[cfg(test)]
pub mod test {
    use hdk::prelude::*;
    use ::fixt::prelude::*;
    use holochain_types::prelude::*;
    use crate::keyset_root::entry::KEYSET_ROOT_CHAIN_INDEX;
    use crate::keyset_root::error::Error;
    use crate::keyset_root::entry::KeysetRootFixturator;

    #[test]
    fn test_validate_update() {
        assert_eq!(
            super::validate_update_entry_keyset_root(fixt!(ValidateData)),
            Error::UpdateAttempted.into(),
        );
    }

    #[test]
    fn test_validate_delete() {
        assert_eq!(
            super::validate_delete_entry_keyset_root(fixt!(ValidateData)),
            Error::DeleteAttempted.into(),
        );
    }

    #[test]
    fn test_validate_create_header() {
        let mut create_header = fixt!(Create);
        create_header.header_seq = KEYSET_ROOT_CHAIN_INDEX + 1;

        assert_eq!(
            super::_validate_create_header(&create_header),
            Error::Position(KEYSET_ROOT_CHAIN_INDEX + 1, KEYSET_ROOT_CHAIN_INDEX).into(),
        );

        create_header.header_seq = KEYSET_ROOT_CHAIN_INDEX;

        assert_eq!(
            super::_validate_create_header(&create_header),
            Ok(ValidateCallbackResult::Valid),
        );
    }

    #[test]
    fn test_validate_create_authorization() {
        let create_header = fixt!(Create);
        let mut proposed_keyset_root = fixt!(KeysetRoot);

        assert_eq!(
            super::_validate_create_authorization(&create_header, &proposed_keyset_root),
            Error::FdaAuthor.into(),
        );

        proposed_keyset_root.first_deepkey_agent = create_header.author.clone();

        let mut mock_hdk = MockHdkT::new();

        mock_hdk.expect_verify_signature()
            .with(mockall::predicate::eq(
                VerifySignature::new_raw(
                    proposed_keyset_root.as_root_pub_key_ref().clone(),
                    proposed_keyset_root.as_fda_pubkey_signed_by_root_key_ref().clone(),
                    proposed_keyset_root.as_first_deepkey_agent_ref().get_raw_32().to_vec(),
                )
            ))
            .times(1)
            .return_const(Ok(false));

        set_hdk(mock_hdk);

        assert_eq!(
            super::_validate_create_authorization(&create_header, &proposed_keyset_root),
            Error::FdaSignature.into(),
        );

        let mut mock_hdk = MockHdkT::new();

        mock_hdk.expect_verify_signature()
            .with(mockall::predicate::eq(
                VerifySignature::new_raw(
                    proposed_keyset_root.as_root_pub_key_ref().clone(),
                    proposed_keyset_root.as_fda_pubkey_signed_by_root_key_ref().clone(),
                    proposed_keyset_root.as_first_deepkey_agent_ref().get_raw_32().to_vec(),
                )
            ))
            .times(1)
            .return_const(Ok(true));

        set_hdk(mock_hdk);

        assert_eq!(
            super::_validate_create_authorization(&create_header, &proposed_keyset_root),
            Ok(ValidateCallbackResult::Valid),
        );
    }

    #[test]
    fn test_validate_create() {
        let mut validate_data = fixt!(ValidateData);
        let mut keyset_root = fixt!(KeysetRoot);
        let mut create_header = fixt!(Create);
        create_header.header_seq = KEYSET_ROOT_CHAIN_INDEX;
        keyset_root.first_deepkey_agent = create_header.author.clone();
        *validate_data.element.as_entry_mut() = ElementEntry::Present(keyset_root.clone().try_into().unwrap());
        *validate_data.element.as_header_mut() = Header::Create(create_header);

        *validate_data.element.as_entry_mut() = ElementEntry::NotStored;

        assert_eq!(
            super::validate_create_entry_keyset_root(validate_data.clone()),
            Ok(ValidateCallbackResult::Invalid("Element missing its KeysetRoot".to_string())),
        );

        *validate_data.element.as_entry_mut() = ElementEntry::Present(keyset_root.clone().try_into().unwrap());

        let mut mock_hdk = hdk::prelude::MockHdkT::new();

        mock_hdk.expect_verify_signature()
            .with(mockall::predicate::eq(
                VerifySignature::new_raw(
                    keyset_root.as_root_pub_key_ref().clone(),
                    keyset_root.as_fda_pubkey_signed_by_root_key_ref().clone(),
                    keyset_root.as_first_deepkey_agent_ref().get_raw_32().to_vec()
                )
            ))
            .times(1)
            .return_const(Ok(true));

        hdk::prelude::set_hdk(mock_hdk);

        assert_eq!(
            super::validate_create_entry_keyset_root(validate_data),
            Ok(ValidateCallbackResult::Valid),
        );
    }
}