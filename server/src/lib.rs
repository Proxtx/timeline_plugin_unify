use {
    rocket::{
        fs::NamedFile, get, http::{ContentType, Status}, post, routes, serde::json::Json, State
    }, serde::{Deserialize, Serialize}, server_api::{
        config::Config, db::{Database, Event}, external::{futures::{self, StreamExt}, types::{api::{APIError, APIResult, CompressedEvent}, available_plugins::AvailablePlugins, external::{chrono::Utc, serde_json}, timing::{TimeRange, Timing}}}, plugin::{PluginData, PluginTrait}
    }, std::{path::PathBuf, sync::Arc}
};

pub struct Plugin {
    plugin_data: PluginData,
}

impl PluginTrait for Plugin {
    async fn new(data: crate::PluginData) -> Self
    where
        Self: Sized,
    {
        Plugin { plugin_data: data }
    }

    fn get_type() -> AvailablePlugins
    where
        Self: Sized,
    {
        AvailablePlugins::timeline_plugin_unify
    }

    fn get_routes() -> Vec<rocket::Route>
    where
        Self: Sized,
    {
        routes![unify_action, get_combine]
    }

    fn get_compressed_events(
        &self,
        query_range: &TimeRange,
    ) -> std::pin::Pin<
        Box<
            dyn futures::Future<Output = APIResult<Vec<CompressedEvent>>>
                + Send,
        >,
    > {
        let filter = Database::generate_range_filter(query_range);
        let plg_filter = Database::generate_find_plugin_filter(Plugin::get_type());
        let filter = Database::combine_documents(filter, plg_filter);
        let database = self.plugin_data.database.clone();
        Box::pin(async move {
            let mut cursor = database
                .get_events::<UnifyRequest>()
                .find(filter, None)
                .await?;
            let mut result = Vec::new();
            while let Some(v) = cursor.next().await {
                let t = v?;
                result.push(CompressedEvent {
                    title: t.event.appName.clone(),
                    time: t.timing,
                    data: serde_json::to_value(t.event).unwrap(),
                })
            }

            Ok(result)
        })
    }
}

#[post("/unify_action", data="<request>")]
async fn unify_action(
    request: Json<AuthorizedUnifyRequest>,
    config: &State<Config>,
    database: &State<Arc<Database>>,
) -> (Status, Json<APIResult<()>>) {
    if request.password != config.password {
        return (Status::Unauthorized, Json(Err(APIError::AuthenticationError)));
    }

    match database
        .register_single_event(&Event {
            timing: Timing::Instant(Utc::now()),
            id: Utc::now().timestamp_millis().to_string(),
            plugin: Plugin::get_type(),
            event: request.request.clone(),
        })
        .await
    {
        Ok(_) => (Status::Ok, Json(Ok(()))),
        Err(e) => {
            server_api::error::error(database.inner().clone(), &e, Some(<Plugin as PluginTrait>::get_type()), &config.error_report_url);
            (Status::InternalServerError, Json(Err(e.into())))
        },
    }
}

#[get("/combine.js")]
pub async fn get_combine() -> (ContentType, Option<NamedFile>) {
    let path = PathBuf::from("../plugins/timeline_plugin_unify/combine.js");
    (ContentType::JavaScript, NamedFile::open(path).await.ok())
}

#[derive(Deserialize)]
struct AuthorizedUnifyRequest {
    password: String,
    request: UnifyRequest
}

#[derive(Deserialize, Serialize, Clone)]
#[allow(non_snake_case)]
struct UnifyRequest {
    method: String,
    appName: String,
    arguments: serde_json::Value,
    unifyUrl: String
}