use { 
    client_api::{external::url::Url, plugin::{PluginData, PluginEventData, PluginTrait}, result::{EventError, EventResult}, style::Style, types::external::serde_json}, leptos::{view, IntoView, View}, serde::{Deserialize, Serialize}, std::str::FromStr
};

pub struct Plugin {
    #[allow(unused)]
    plugin_data: PluginData,
}

impl PluginTrait for Plugin {
    async fn new(data: PluginData) -> Self
        where
            Self: Sized {
            Plugin {
                plugin_data: data
            }
    }

    fn get_component(&self, data: PluginEventData) -> EventResult<Box<dyn FnOnce() -> leptos::View>> {
        let data = data.get_data::<UnifyRequest>()?;
        let iframe_id = rand::random::<u64>();
        let mut url = match Url::from_str(&data.unifyUrl) {
            Ok(v) => v,
            Err(e) => return Err(EventError::FaultyInitData(format!("{}", e)))
        };
        url = url.join("actionCreator").unwrap();
        let src = format!("
        window.run_{} = async () =>  {{
          setTimeout(window.run_{}, 50);
        }};
        import {{genModule, genCombine as genCombineIframe}} from \"/api/plugin/timeline_plugin_unify/combine.js\"
        if (!Element.prototype.enableCombine)
        Element.prototype.enableCombine = async function (module) {{
        if (this.tagName != \"IFRAME\") return;
        while (!this.contentWindow?.postMessage) {{
          await new Promise((r) => setTimeout(r, 100));
        }}
        let latestMessage;
        window.addEventListener(\"message\", (ev) => {{
          latestMessage = ev.data;
        }});
        let msg = \"ping\" + Math.floor(Math.random() * 1000);
        while (
          !latestMessage ||
          latestMessage.substring(4) != msg.substring(4)
        ) {{
          this.contentWindow.postMessage(msg, \"*\");
          await new Promise((r) => setTimeout(r, 100));
        }}

        delete this.combine;

        while (!this.combine)
          try {{
            this.combine = await genCombineIframe(
              module,
              genModule,
              this.contentWindow
            );
          }} catch (e) {{
            console.log(
              \"Combine loading failed. Assuming the module has not loaded yet. Retry. Error:\",
              e
            );
            await new Promise((r) => setTimeout(r, 100));
          }}
        return this.combine;
      }};
        window.run_{} = async () => {{
          
          let iframe = await document.getElementById(\"{}\");
          await iframe.enableCombine(\"interaction\");
        let data = JSON.parse(\"{{\\\"appName\\\": {}, \\\"method\\\": {}, \\\"arguments\\\": {}}}\");
        iframe.combine.importAction(data);
        do {{
      let dimensions = await iframe.combine.size();
      iframe.style.height = dimensions.height + \"px\";
    }} while(await iframe.combine.resizeObserver() || true) }}", iframe_id, iframe_id, iframe_id, iframe_id, sanitize(&serde_json::to_string(&data.appName).unwrap()), sanitize(&serde_json::to_string(&data.method).unwrap()), sanitize(&serde_json::to_string(&data.arguments).unwrap()));
        Ok(Box::new(move || -> View {
            view! {
                <iframe
                    id=iframe_id
                    src=url.to_string()
                    style:width="100%"
                    style:border="none"
                    onload=format!("window.run_{}()", iframe_id)
                >
                    Loading
                </iframe>
                <script type="module">{src}</script>
            }.into_view()
        }))
    }

    fn get_style(&self) -> Style {
        Style::Custom("#507DBC".to_string(), "#BBD1EA".to_string(), "var(--lightColor)".to_string())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
struct UnifyRequest {
    pub appName: String,
    pub method: String,
    pub arguments: serde_json::Value,
    pub unifyUrl: String
}

fn sanitize (str: &str) -> String {
    str.replace('\\', "\\\\").replace('"', "\\\"")
}