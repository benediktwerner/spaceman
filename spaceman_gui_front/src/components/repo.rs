use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use spaceman_types::repo::{RepoView, ServiceView, MethodView};

#[derive(Properties, PartialEq)]
pub struct RepoProps {
    pub repo_view: Option<RepoView>,
    pub on_new_tab: Callback<(usize, usize)>,
}

#[function_component]
pub fn Repo(props: &RepoProps) -> Html {
    let query = use_state_eq::<Option<String>, _>(|| None);
    
    let content = props.repo_view.as_ref().map(
        |repo_view| repo_view.services.clone()
    ).unwrap_or_else(|| Vec::new()).into_iter().enumerate().map(|(service_idx, service_view)| {
        html!{ <Service service_view={ service_view } query={ (*query).clone() } on_new_tab={
            Callback::from({
                let cb = props.on_new_tab.clone();
                move |method_idx: usize| {
                    cb.emit((service_idx, method_idx));
                }
            }) 
        }/> }
    }).collect::<Html>();

    let oninput = Callback::from(move |ev: InputEvent| {
        let raw = ev.target_unchecked_into::<HtmlInputElement>().value();
        query.set(if raw.is_empty() {
            None
        } else {
            Some(raw)
        });
    });

    html! {
        <div class="repo">
            <input type="text" placeholder="Search" {oninput}/>
            <div class="content">
                {content}
            </div>
        </div>
    }
}

#[derive(PartialEq, Properties)]
struct ServiceProps {
    service_view: ServiceView,
    query: Option<String>,
    on_new_tab: Callback<usize>,
}

#[function_component]
fn Service(props: &ServiceProps) -> Html {
    let methods_n = props.service_view.methods.len();
    let mut methods: Vec<_> = props.service_view.methods.iter().enumerate().filter_map(|(method_idx, method_view)| {
        match &props.query {
            Some(query) if !method_view.name.to_lowercase().contains(&query.to_lowercase()) => {
                None
            },
            _ => {
                Some((method_view.clone(), method_idx, false))
            }
        }
    }).collect();

    if let Some((_, _, is_last)) = methods.last_mut() {
        *is_last = true;
    } else {
        // There are no methods
        return Html::default();
    }

    html! {
        <div class="service">
            <div class="name">{ props.service_view.full_name.clone() }</div>
            {
                for methods.into_iter().map(|(method_view, method_idx, is_last)| {
                    html!{ <Method {method_view} {is_last} on_new_tab={
                        Callback::from({
                            let cb = props.on_new_tab.clone() ;
                            move |_| {
                                cb.emit(method_idx);
                            }
                        }) 
                    }/> }
                })
            }
        </div>
    }
}

#[derive(PartialEq, Properties)]
struct MethodProps {
    is_last: bool,
    method_view: MethodView,
    on_new_tab: Callback<()>,
}

#[function_component]
fn Method(props: &MethodProps) -> Html {
    let onclick = Callback::from({
        let cb = props.on_new_tab.clone();
        move |_| {
            cb.emit(())
        }
    });

    html! {
        <div class="method">
            {
                if props.is_last {
                    html!{ <img class="uplink" src="img/method_uplink_last.svg"/> }
                } else {
                    html!{ <img class="uplink" src="img/method_uplink.svg"/> }
                }
            }
            <div class="name" { onclick }>{ props.method_view.name.clone() }</div>
        </div>
    }
}
