macro_rules! unsupported_remote_extensions {
    () => {
        fn verify_bundle_executable<'a>(
            &'a self,
            _: &'a crate::remote::BundleStageRequest,
            _: crate::remote::BundleExecutableLocation,
            _: crate::remote::ArtifactRole,
        ) -> crate::remote::RemoteFuture<'a, crate::remote::ChannelOutcome<()>> {
            Box::pin(async { Err(crate::remote::RemoteError::ChannelRejected) })
        }
        fn stage_bundle<'a>(
            &'a self,
            _: &'a crate::remote::BundleStageRequest,
        ) -> crate::remote::RemoteFuture<
            'a,
            crate::remote::ChannelOutcome<crate::remote::BundleStageResult>,
        > {
            Box::pin(async { Err(crate::remote::RemoteError::ChannelRejected) })
        }
        fn read_bundle_selection<'a>(
            &'a self,
            _: &'a std::path::Path,
        ) -> crate::remote::RemoteFuture<'a, crate::remote::ChannelOutcome<Option<Vec<u8>>>> {
            Box::pin(async { Err(crate::remote::RemoteError::ChannelRejected) })
        }
        fn open_hub_peer<'a>(
            &'a self,
            _: &'a crate::remote::HubPeerRequest,
        ) -> crate::remote::RemoteFuture<'a, crate::remote::OpenedHubPeerChannel> {
            Box::pin(async { Err(crate::remote::RemoteError::ChannelRejected) })
        }
    };
}

pub(crate) use unsupported_remote_extensions;
