use crate::{
    pow::{self, HeaderHasher},
    proto::{
        velkard_message::Payload, GetBlockTemplateRequestMessage, GetInfoRequestMessage,
        NotifyBlockAddedRequestMessage, NotifyNewBlockTemplateRequestMessage, RpcBlock, SubmitBlockRequestMessage,
        VelkardMessage,
    },
    Hash,
};

impl VelkardMessage {
    #[must_use]
    #[inline(always)]
    pub fn get_info_request() -> Self {
        VelkardMessage { payload: Some(Payload::GetInfoRequest(GetInfoRequestMessage {})) }
    }
    #[must_use]
    #[inline(always)]
    pub fn notify_block_added() -> Self {
        VelkardMessage { payload: Some(Payload::NotifyBlockAddedRequest(NotifyBlockAddedRequestMessage {})) }
    }
    #[must_use]
    #[inline(always)]
    pub fn submit_block(block: RpcBlock) -> Self {
        VelkardMessage {
            payload: Some(Payload::SubmitBlockRequest(SubmitBlockRequestMessage {
                block: Some(block),
                allow_non_daa_blocks: false,
            })),
        }
    }
}

impl From<GetInfoRequestMessage> for VelkardMessage {
    #[inline(always)]
    fn from(a: GetInfoRequestMessage) -> Self {
        VelkardMessage { payload: Some(Payload::GetInfoRequest(a)) }
    }
}
impl From<NotifyBlockAddedRequestMessage> for VelkardMessage {
    #[inline(always)]
    fn from(a: NotifyBlockAddedRequestMessage) -> Self {
        VelkardMessage { payload: Some(Payload::NotifyBlockAddedRequest(a)) }
    }
}

impl From<GetBlockTemplateRequestMessage> for VelkardMessage {
    #[inline(always)]
    fn from(a: GetBlockTemplateRequestMessage) -> Self {
        VelkardMessage { payload: Some(Payload::GetBlockTemplateRequest(a)) }
    }
}

impl From<NotifyNewBlockTemplateRequestMessage> for VelkardMessage {
    fn from(a: NotifyNewBlockTemplateRequestMessage) -> Self {
        VelkardMessage { payload: Some(Payload::NotifyNewBlockTemplateRequest(a)) }
    }
}

impl RpcBlock {
    #[must_use]
    #[inline(always)]
    pub fn block_hash(&self) -> Option<Hash> {
        let mut hasher = HeaderHasher::new();
        pow::serialize_header(&mut hasher, self.header.as_ref()?, false);
        Some(hasher.finalize())
    }
}
