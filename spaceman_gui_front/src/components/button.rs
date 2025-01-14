use yew::prelude::*;

#[derive(PartialEq)]
pub enum ButtonKind {
    Standard,
    Green,
    Red,
    Blue,
    Cyan,
}

#[derive(Properties, PartialEq)]
pub struct ButtonProps {
    pub text: String,
    pub icon: Option<AttrValue>,
    pub class: Option<Classes>,
    pub style: Option<AttrValue>,
    #[prop_or(ButtonKind::Standard)]
    pub kind: ButtonKind,
    pub onclick: Option<Callback<MouseEvent>>,

    #[prop_or(false)]
    pub has_led: bool,
    #[prop_or(false)]
    pub is_led_lit: bool,
}

#[function_component]
pub fn Button(props: &ButtonProps) -> Html {
    let class = classes!{
        "button",
        props.class.clone(),
        match props.kind {
            ButtonKind::Standard => None,
            ButtonKind::Green => Some("green"),
            ButtonKind::Red => Some("red"),
            ButtonKind::Blue => Some("blue"),
            ButtonKind::Cyan => Some("cyan"),
        }
    };

    let onclick = props.onclick.clone();

    html!{
        <button style={ props.style.clone() } { class } { onclick }>
            {
                if props.has_led {
                    Some(html! {
                        <div class={classes!("led", if props.is_led_lit {Some("lit")} else {None})}></div>
                    })
                } else {
                    None
                }
            }
            {
                if let Some(icon) = props.icon.clone() {
                    html!{<img src={ icon }/>}
                } else {
                    html!{}
                }
            }
            { props.text.clone() }
        </button>
    }
}
