use std::{
    fs::{self, OpenOptions},
    process::{self, Command, Stdio},
};

use teloxide::{
    prelude::*,
    types::{KeyboardButton, KeyboardMarkup, ParseMode},
};

use exitcode;
use serde::{Deserialize, Serialize};
use toml;

const CONFIG_DATA: &str = r#""token" = ""
"#;

use simplelog::*;

#[tokio::main]
async fn main() {
    let logger_config = simplelog::ConfigBuilder::new()
        .set_time_format_custom(format_description!(
            "[day].[month].[year]  [hour]:[minute]:[second]"
        ))
        //.set_time_offset_to_local()
        //.unwrap()
        .build();
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            logger_config.clone(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Info,
            logger_config,
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .append(true)
                .open("log_data.log")
                .unwrap(),
        ),
    ])
    .unwrap();

    log::info!("Starting bot...");

    let token = check_config().token;

    let bot = Bot::new(token);

    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        if let Some(text) = msg.text() {
            match text {
                "info" => {
                    let res = &outside_output();
                    //let res = make_json();
                    let vec_instance: Result<Vec<Instance>, serde_json::Error> =
                        serde_json::from_str(res);
                    match vec_instance {
                        Ok(vec_instance) => {
                            //dbg!(&vec_instance);

                            let v_st_instances: Vec<String> = vec_instance
                                .iter()
                                .map(|x| {
                                    let v_strings: Vec<String> = x
                                        .backups
                                        .iter()
                                        .map(|b| {
                                            format!(
                                                "-------------------------------------------------------\nid: {}\nrecovery time: {}\nbackup mode: {}\nstatus: {}",
                                                b.id,
                                                b.recovery_time,
                                                b.backup_mode,
                                                //b.parent_backup_id.clone().unwrap_or("none".to_string()),
                                                b.status
                                            )
                                        })
                                        .collect();

                                    format!("Instance: <b>{}</b>\n{}", x.instance, v_strings.join("\n"))
                                })
                                .collect();

                            for text in v_st_instances {
                                let keyboard = make_keyboard();
                                bot.send_message(msg.chat.id, text)
                                    .parse_mode(ParseMode::Html)
                                    .reply_markup(keyboard)
                                    .await?;
                            }
                        }
                        Err(err) => {
                            let keyboard = make_keyboard();
                            bot.send_message(msg.chat.id, "Не удалось обработать ответ сервера")
                                .reply_markup(keyboard)
                                .await?;

                            let keyboard = make_keyboard();
                            bot.send_message(msg.chat.id, err.to_string())
                                .reply_markup(keyboard)
                                .await?;
                        }
                    }
                }

                "/start" | "check_bot" => {
                    let keyboard = make_keyboard();
                    bot.send_message(msg.chat.id, "Бот запущен")
                        .reply_markup(keyboard)
                        .await?;
                }

                _ => {
                    let keyboard = make_keyboard();
                    bot.send_message(msg.chat.id, "Нет такой команды")
                        .reply_markup(keyboard)
                        .await?;
                }
            }
        }

        Ok(())
    })
    .await;
}

fn make_keyboard() -> KeyboardMarkup {
    let mut keyboard: Vec<Vec<KeyboardButton>> = vec![];

    let commands = ["info", "check_bot"];

    for commands_row in commands.chunks(1) {
        let row = commands_row
            .iter()
            .map(|&command| KeyboardButton::new(command.to_owned()))
            .collect();

        keyboard.push(row);
    }
    KeyboardMarkup::new(keyboard)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Instance {
    instance: String,
    backups: Vec<Backup>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Backup {
    pub id: String,
    #[serde(rename(deserialize = "recovery-time"))]
    pub recovery_time: String,
    // #[serde(rename(deserialize = "parent-backup-id"))]
    // pub parent_backup_id: Option<String>,
    #[serde(rename(deserialize = "backup-mode"))]
    pub backup_mode: String,
    pub status: String,
}

fn outside_output() -> String {
    let output = Command::new("pg_probackup")
        .arg("show")
        .arg("--instance=odo")
        .arg("--format=json")
        .stdout(Stdio::piped())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    format!("{}", stdout)
}

#[derive(Deserialize)]
struct Config {
    token: String,
}

fn check_config() -> Config {
    let config_file = match fs::read_to_string("config.toml") {
        Ok(file) => file,
        Err(_) => {
            fs::write("./config.toml", CONFIG_DATA)
                .expect("НЕ УДАЛОСЬ ЗАПИСАТЬ СОЗДАТЬ config.toml, ПРОВЕРЬТЕ ПРАВА");
            log::warn!(
                "Не заполнен config, необходимо заполнить config.toml и перезапустить службу"
            );
            process::exit(exitcode::OK);
        }
    };

    let config: Config = toml::from_str(&config_file).expect("НЕПРАВИЛЬНО ЗАПОЛНЕН config.toml");
    if config.token.is_empty() {
        log::warn!("Не все поля config файла заполнены");
        process::exit(exitcode::OK);
    }
    config
}
