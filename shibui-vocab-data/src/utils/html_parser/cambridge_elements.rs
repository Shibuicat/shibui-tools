use scraper::{ElementRef, Html};

pub struct WordPage<'a> {
    pub content: &'a Html,
}

impl<'a> WordPage<'a> {
    //There is a possibility that the word does not exist.
    pub fn new(html: &Html) -> anyhow::Result<Self> {
        //validate the page before parsing
    }
}

pub struct WordClassSection<'a> {
    //the word class ele
    pub inner_html_ele: &'a ElementRef<'a>,
}

impl<'a> WordClassSection<'a> {}

pub struct WordClassHeaderSection<'a> {
    pub inner_html_ele: &'a ElementRef<'a>,
}
