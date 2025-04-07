mod peeker;

pub use self::peeker::TextPeeker;
use super::{Text, TextStream};

pub trait IntoText {
    fn into_text(self) -> impl Text + Send;
}

impl<T> IntoText for T
where
    T: TextStream + Send,
{
    fn into_text(self) -> impl Text + Send
    where
        Self: Sized + Send,
    {
        TextPeeker::new(self)
    }
}

pub trait IntoTextNoSend {
    fn into_text_no_send(self) -> impl Text;
}

impl<T> IntoTextNoSend for T
where
    T: TextStream,
{
    fn into_text_no_send(self) -> impl Text
    where
        Self: Sized,
    {
        TextPeeker::new(self)
    }
}
