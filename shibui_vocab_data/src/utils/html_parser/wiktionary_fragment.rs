use scraper::{ElementRef, Node};

const KEPT_ATTRIBUTES: [&str; 7] = ["class", "lang", "href", "src", "title", "id", "type"];
const DROPPED_TAGS: [&str; 4] = ["link", "style", "script", "meta"];
const VOID_TAGS: [&str; 6] = ["br", "hr", "img", "source", "wbr", "input"];

pub fn clean_html(element: ElementRef) -> String {
    let mut output = String::new();
    write_element(element, &mut output);
    output
}

fn write_element(element: ElementRef, output: &mut String) {
    let name = element.value().name();
    if DROPPED_TAGS.contains(&name) {
        return;
    }

    output.push('<');
    output.push_str(name);
    element
        .value()
        .attrs()
        .filter(|(attribute, _)| KEPT_ATTRIBUTES.contains(attribute))
        .for_each(|(attribute, value)| {
            output.push_str(&format!(" {attribute}=\"{}\"", escape_attribute(value)));
        });
    output.push('>');

    if VOID_TAGS.contains(&name) {
        return;
    }

    element.children().for_each(|node| match node.value() {
        Node::Text(text) => output.push_str(&escape_text(text)),
        Node::Element(_) => {
            if let Some(child) = ElementRef::wrap(node) {
                write_element(child, output);
            }
        }
        _ => {}
    });
    output.push_str(&format!("</{name}>"));
}

fn escape_text(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn escape_attribute(value: &str) -> String {
    escape_text(value).replace('"', "&quot;")
}
