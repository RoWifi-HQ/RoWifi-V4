mod update;

use rowifi_framework::{command::Command, Framework};

use self::update::update_func;

pub fn user_config(framework: &mut Framework) {
    let update_cmd = Command::builder()
        .node()
        .name("update")
        .handler(update_func);

    framework.add_command(update_cmd);
}
