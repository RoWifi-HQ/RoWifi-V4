pub mod context;

use rowifi_models::discord::application::command::CommandOption;

pub struct Interaction {
    pub data: Vec<CommandOption>,
}

pub struct Framework {

}