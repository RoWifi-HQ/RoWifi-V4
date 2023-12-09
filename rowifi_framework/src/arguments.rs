use rowifi_models::discord::application::interaction::application_command::CommandDataOption;

#[allow(dead_code)]
#[derive(Debug)]
pub enum ArgumentError {
    BadArgument
}

pub trait Arguments {
    fn from_interaction(options: &[CommandDataOption]) -> Result<Self, ArgumentError> where Self: Sized;
}

impl Arguments for () {
    fn from_interaction(_: &[CommandDataOption]) -> Result<Self, ArgumentError> {
        Ok(())
    }
}

impl<T: Arguments> Arguments for (T, ) {
    fn from_interaction(options: &[CommandDataOption]) -> Result<Self, ArgumentError> {
        match T::from_interaction(options) {
            Ok(a) => Ok((a,)),
            Err(err) => Err(err)
        }
    }
}

impl<T: Arguments> Arguments for Option<T> {
    fn from_interaction(options: &[CommandDataOption]) -> Result<Self, ArgumentError> {
        Ok(T::from_interaction(options).ok())
    }
}