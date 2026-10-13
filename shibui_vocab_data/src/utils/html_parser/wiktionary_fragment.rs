use scraper::{ElementRef, Node};

const KEPT_ATTRIBUTES: [&str; 7] = ["class", "lang", "href", "src", "title", "id", "type"];
const DROPPED_TAGS: [&str; 4] = ["link", "style", "script", "meta"];
const VOID_TAGS: [&str; 6] = ["br", "hr", "img", "source", "wbr", "input"];

pub fn clean_html(element: ElementRef) -> String {
    clean_html_with(element, &KEPT_ATTRIBUTES, &|_| false)
}

pub fn clean_html_with(element: ElementRef, kept_attributes: &[&str], skip: &dyn Fn(ElementRef) -> bool) -> String {
    let mut output = String::new();
    write_element(element, kept_attributes, skip, &mut output);
    output
}

fn write_element(element: ElementRef, kept_attributes: &[&str], skip: &dyn Fn(ElementRef) -> bool, output: &mut String) {
    let name = element.value().name();
    if DROPPED_TAGS.contains(&name) || skip(element) {
        return;
    }

    output.push('<');
    output.push_str(name);
    element
        .value()
        .attrs()
        .filter(|(attribute, _)| kept_attributes.contains(attribute))
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
                write_element(child, kept_attributes, skip, output);
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
