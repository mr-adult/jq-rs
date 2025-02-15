use crate::{fuse::FuseOnErr, stream_context::StreamContext, JQStream, SanitizedJQStream, Scope};

/// A struct that transforms any [`JQStream`] into a
/// [`crate::SanitizedJQStream`]
pub struct Sanitized<Stream>
where
    Stream: JQStream,
{
    stream: StreamContext<FuseOnErr<Stream>>,
}

impl<Stream> Sanitized<Stream>
where
    Stream: JQStream,
{
    pub(crate) fn new(stream: Stream) -> Self {
        Self {
            stream: StreamContext::new(FuseOnErr::new(stream)),
        }
    }

    pub fn get_path(&self) -> &[Scope] {
        self.stream.get_path()
    }
}

impl<Stream> Iterator for Sanitized<Stream>
where
    Stream: JQStream,
{
    type Item = crate::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.stream.next()
    }
}

impl<Stream> SanitizedJQStream for Sanitized<Stream> where Stream: JQStream {}
