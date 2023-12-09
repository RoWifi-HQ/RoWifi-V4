use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};
use tower::Service;

use crate::{Request, error::FrameworkError, context::CommandContext, arguments::Arguments};

pub trait Handler<T, R>: 'static
where
    R: Future<Output = Result<(), FrameworkError>>,
{
    fn call(&self, ctx: CommandContext, param: T) -> R;
}

pub struct CommandHandler<F, T, R>
where
    F: Handler<T, R>,
    T: Arguments,
    R: Future<Output = Result<(), FrameworkError>>,
{
    hnd: F,
    _p: PhantomData<(T, R)>,
}

impl<F, T, R> CommandHandler<F, T, R>
where
    F: Handler<T, R>,
    T: Arguments,
    R: Future<Output = Result<(), FrameworkError>>,
{
    pub fn new(hnd: F) -> Self {
        Self {
            hnd,
            _p: PhantomData,
        }
    }
}

impl<F, T, R> Service<Request> for CommandHandler<F, T, R>
where
    F: Handler<T, R>,
    T: Arguments,
    R: Future<Output = Result<(), FrameworkError>> + Send + 'static,
{
    type Response = ();
    type Error = FrameworkError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        match T::from_interaction(&req.interaction.data) {
            Ok(args) => {
                let fut = self.hnd.call(req.context, args);
                Box::pin(fut)
            },
            Err(err) => {
                let fut = async move { Err(err.into()) };
                Box::pin(fut)
            }
        }
    }
}

impl<F, R> Handler<(), R> for F
where
    F: Fn(CommandContext) -> R + 'static,
    R: Future<Output = Result<(), FrameworkError>>,
{
    fn call(&self, ctx: CommandContext, (): ()) -> R {
        (self)(ctx)
    }
}

impl<F, T, R> Handler<(T,), R> for F
where
    F: Fn(CommandContext, T) -> R + 'static,
    R: Future<Output = Result<(), FrameworkError>>,
{
    fn call(&self, ctx: CommandContext, (param,): (T,)) -> R {
        (self)(ctx, param)
    }
}
