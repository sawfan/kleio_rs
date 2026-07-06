//! Browser-friendly async repository boundary for `TimelineDocument` values.
//!
//! Native file repositories can stay synchronous, while browser/worker/OPFS or
//! future remote backends often need asynchronous APIs. This module provides a
//! small boxed-future trait without adding an async-trait dependency.

use std::future::Future;
use std::pin::Pin;

use crate::pack::TimelineDocument;
use crate::timeline_repository::{TimelineDocumentRepository, TimelineRepositoryError};

pub type TimelineRepositoryFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, TimelineRepositoryError>> + 'a>>;

pub trait AsyncTimelineDocumentRepository {
    fn load(&self) -> TimelineRepositoryFuture<'_, TimelineDocument>;
    fn save<'a>(&'a self, document: &'a TimelineDocument) -> TimelineRepositoryFuture<'a, ()>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsyncFromSyncRepository<R> {
    inner: R,
}

impl<R> AsyncFromSyncRepository<R> {
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    pub fn inner(&self) -> &R {
        &self.inner
    }

    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R> AsyncTimelineDocumentRepository for AsyncFromSyncRepository<R>
where
    R: TimelineDocumentRepository,
{
    fn load(&self) -> TimelineRepositoryFuture<'_, TimelineDocument> {
        Box::pin(std::future::ready(self.inner.load()))
    }

    fn save<'a>(&'a self, document: &'a TimelineDocument) -> TimelineRepositoryFuture<'a, ()> {
        Box::pin(std::future::ready(self.inner.save(document)))
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    use super::*;
    use crate::sample_timeline_document;

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn async_adapter_wraps_sync_json_repository() {
        let path = std::env::temp_dir().join(format!(
            "kleio-async-timeline-document-{}.json",
            std::process::id()
        ));
        let sync_repo = crate::file::JsonTimelineDocumentFileRepository::new(&path);
        let repo = AsyncFromSyncRepository::new(sync_repo);
        let document = sample_timeline_document();

        let save_result = poll_ready(repo.save(&document));
        assert!(save_result.is_ok());

        let loaded = poll_ready(repo.load()).expect("load document");
        let _ = std::fs::remove_file(path);

        assert_eq!(loaded.packs.len(), document.packs.len());
        assert_eq!(loaded.active_pack_ids, document.active_pack_ids);
    }

    fn poll_ready<T>(
        mut future: TimelineRepositoryFuture<'_, T>,
    ) -> Result<T, TimelineRepositoryError> {
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        match Future::poll(future.as_mut(), &mut context) {
            Poll::Ready(result) => result,
            Poll::Pending => panic!("test future unexpectedly pending"),
        }
    }
}
