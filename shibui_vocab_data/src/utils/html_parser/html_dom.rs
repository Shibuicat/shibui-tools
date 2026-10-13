use scraper::{ElementRef, Node, Selector};

pub fn child_elements<'a>(element: ElementRef<'a>) -> impl Iterator<Item = ElementRef<'a>> {
    element.children().filter_map(ElementRef::wrap)
}

pub fn has_class(element: ElementRef, class: &str) -> bool {
    element.value().classes().any(|candidate| candidate == class)
}

pub fn is_heading(element: ElementRef) -> bool {
    matches!(element.value().name(), "h2" | "h3" | "h4" | "h5" | "h6")
}

pub fn heading_of(section: ElementRef) -> Option<ElementRef> {
    child_elements(section).find(|child| is_heading(*child))
}

pub fn child_sections<'a>(section: ElementRef<'a>) -> impl Iterator<Item = ElementRef<'a>> {
    child_elements(section).filter(|child| child.value().name() == "section")
}

pub fn text_without(element: ElementRef, skip: &dyn Fn(ElementRef) -> bool) -> String {
    let mut collected = String::new();
    collect_text(element, skip, &mut collected);
    tidy(&collected)
}

pub fn text_of(element: ElementRef) -> String {
    text_without(element, &|_| false)
}

fn collect_text(element: ElementRef, skip: &dyn Fn(ElementRef) -> bool, into: &mut String) {
    element.children().for_each(|node| match node.value() {
        Node::Text(text) => into.push_str(text),
        Node::Element(_) => {
            if let Some(child) = ElementRef::wrap(node) {
                if !skip(child) {
                    collect_text(child, skip, into);
                }
            }
        }
        _ => {}
    });
}

pub fn tidy(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" ,", ",")
        .replace(" .", ".")
        .replace(" ;", ";")
        .replace(" :", ":")
        .replace("( ", "(")
        .replace(" )", ")")
        .replace(" ]", "]")
        .replace("[ ", "[")
}

pub fn first_descendant<'a>(
    element: ElementRef<'a>,
    matches: &dyn Fn(ElementRef<'a>) -> bool,
) -> Option<ElementRef<'a>> {
    element
        .descendants()
        .filter_map(ElementRef::wrap)
        .find(|candidate| matches(*candidate))
}

pub fn select_all<'a>(scope: ElementRef<'a>, css: &str) -> Vec<ElementRef<'a>> {
    Selector::parse(css)
        .map(|selector| scope.select(&selector).collect())
        .unwrap_or_default()
}

pub fn select_first<'a>(scope: ElementRef<'a>, css: &str) -> Option<ElementRef<'a>> {
    select_all(scope, css).into_iter().next()
}

pub fn has_ancestor_class(element: ElementRef, class: &str) -> bool {
    element
        .ancestors()
        .filter_map(ElementRef::wrap)
        .any(|ancestor| has_class(ancestor, class))
}

pub fn path_of(url: &str) -> String {
    url.split_once("://")
        .and_then(|(_, rest)| rest.find('/').map(|slash| rest[slash..].to_owned()))
        .unwrap_or_else(|| url.to_owned())
}
