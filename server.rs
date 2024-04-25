use {
    crate::{
        config::Config, db::{Database, Event}, Plugin as _, PluginData
    },
    chrono::Utc,
    futures::StreamExt,
    rocket::{
        fs::NamedFile, get, http::{ContentType, Status}, post, routes, serde::json::Json, State
    },
    serde::{Deserialize, Serialize},
    std::{path::PathBuf, sync::Arc},
    types::{
        api::{APIError, APIResult, CompressedEvent},
        timing::Timing,
    },
};

pub struct Plugin {
    plugin_data: PluginData,
}

impl crate::Plugin for Plugin {
    async fn new(data: crate::PluginData) -> Self
    where
        Self: Sized,
    {
        Plugin { plugin_data: data }
    }

    fn get_type() -> types::api::AvailablePlugins
    where
        Self: Sized,
    {
        types::api::AvailablePlugins::timeline_plugin_unify
    }

    fn get_routes() -> Vec<rocket::Route>
    where
        Self: Sized,
    {
        routes![unify_action, get_combine]
    }

    fn get_compressed_events(
        &self,
        query_range: &types::timing::TimeRange,
    ) -> std::pin::Pin<
        Box<
            dyn futures::Future<Output = types::api::APIResult<Vec<types::api::CompressedEvent>>>
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
                    data: Box::new(t.event),
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
            crate::error::error(database.inner().clone(), &e, Some(<Plugin as crate::Plugin>::get_type()), &config.error_report_url);
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