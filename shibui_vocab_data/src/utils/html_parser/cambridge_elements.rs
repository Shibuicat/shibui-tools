use anyhow::bail;
use scraper::{selectable::Selectable, ElementRef, Html, Selector};

use crate::scraper::{
    Class, ClassDefinition, Region, WordClass, WordContext, WordDefinition, WordExplanation,
    WordPronounce, WordUsageExample,
};

pub struct WordPage<'a> {
    pub content: &'a Html,
}

impl<'a> WordPage<'a> {
    //There is a possibility that the word does not exist.
    pub fn new(html: &'a Html) -> anyhow::Result<Self> {
        //document.querySelector(".ddef_h .def.ddef_d.db .usage.dusage")
        let init = Self { content: html };
        if init.word_class_sections().len() == 0 {
            bail!("Word doesn't exist");
        }

        let selector = Selector::parse(".ddef_h .def.ddef_d.db .usage.dusage").unwrap();
        if init.content.select(&selector).count() == 1{
            println!("other tense of the word found");
           bail!("Word doesn't exist"); 
        }

        println!("Len is {}", init.word_class_sections().len());
        Ok(init)
    }

    fn word_class_sections(&'a self) -> Vec<WordClassSection<'a>> {
        let selector = Selector::parse(".pr.entry-body__el").unwrap();
        self.content
            .select(&selector)
            .map(|ele| WordClassSection::new(ele, &self))
            .collect()
    }

    pub fn get_word_definition(&self) -> anyhow::Result<WordDefinition> {
        let word_classes = self.word_class_sections();
        if word_classes.len() == 0 {
            bail!("Word not found");
        }

        let word = word_classes.first().unwrap().get_current_word();
        let word_definition = WordDefinition {
            word,
            classes: word_classes
                .iter()
                .map(|class| class.get_word_class_definition())
                .collect(),
        };

        Ok(word_definition)
    }
}

pub struct WordClassSection<'a> {
    //the word class ele
    pub inner_html_ele: ElementRef<'a>,
    word_page: &'a WordPage<'a>,
}

impl<'a> WordClassSection<'a> {
    pub fn new(inner_html_ele: ElementRef<'a>, word_page: &'a WordPage) -> Self {
        Self {
            word_page,
            inner_html_ele,
        }
    }

    pub fn get_current_word(&self) -> String {
        let header = self.header();
        header.get_word()
    }

    pub fn get_word_class_definition(&self) -> WordClass {
        let header = self.header();

        WordClass {
            class_name: header.get_class(),
            pronounces: header.get_pronounces(),
            definitions: self
                .definitions()
                .iter()
                .map(|x| x.get_definition())
                .collect(),
        }
    }

    pub fn definitions(&'a self) -> Vec<ClassDefinitionSection<'a>> {
        let selector = Selector::parse(".pos-body").unwrap();
        self.inner_html_ele
            .select(&selector)
            .map(|x| ClassDefinitionSection::new(x, &self))
            .collect()
    }

    pub fn header(&'a self) -> WordClassHeaderSection<'a> {
        let selector = Selector::parse(".pos-header.dpos-h").unwrap();
        let header: WordClassHeaderSection<'a> = self
            .inner_html_ele
            .select(&selector)
            .map(|ele| WordClassHeaderSection::new(ele, self))
            .next()
            .unwrap();

        header
    }
}

pub struct WordClassHeaderSection<'a> {
    pub inner_html_ele: ElementRef<'a>,
    word_class: &'a WordClassSection<'a>,
}

impl<'a> WordClassHeaderSection<'a> {
    pub fn new(inner_html_ele: ElementRef<'a>, word_class: &'a WordClassSection) -> Self {
        Self {
            inner_html_ele,
            word_class,
        }
    }

    pub fn get_word(&self) -> String {
        let selector = Selector::parse(".hw.dhw").unwrap();
        let header_ele = self.inner_html_ele.select(&selector).next().unwrap();
        let text = header_ele.text().next();
        return text.unwrap().to_owned();
    }

    pub fn get_class(&self) -> Class {
        let selector = Selector::parse(".pos.dpos").unwrap();
        let class_ele = self.inner_html_ele.select(&selector).next().unwrap();
        let text = class_ele.text().next().unwrap();
        text.into()
    }

    pub fn get_pronounces(&self) -> Vec<WordPronounce> {
        let mut result = vec![];

        let uk_ipa = self.get_uk_ipa();

        if uk_ipa.is_some() {
            result.push(WordPronounce {
                ipa: uk_ipa.unwrap(),
                link: self.get_uk_sound_link().unwrap(),
                region: Region::UK,
            })
        }

        result.push(WordPronounce {
            ipa: self.get_us_ipa().map_or_else(|| "".to_string(), |x| x),
            link: self.get_us_sound_link(),
            region: Region::US,
        });

        result
    }

    fn get_uk_sound_link(&self) -> Option<String> {
        let selector = Selector::parse("audio source").unwrap();
        let source_tag = self.inner_html_ele.select(&selector).next();
        if source_tag.is_some() {
            let src = source_tag.unwrap().attr("src").unwrap();

            return Some(src.to_string());
        }

        return None;
    }

    fn get_us_sound_link(&self) -> String {
        let selector = Selector::parse(".us.dpron-i audio source").unwrap();
        let source_tag = self.inner_html_ele.select(&selector).next().unwrap();

        let src = source_tag.attr("src").unwrap();
        return src.to_string();
    }

    fn get_uk_ipa(&self) -> Option<String> {
        let selector = Selector::parse(".uk.dpron-i .ipa.dipa.lpr-2.lpl-1").unwrap();
        let ipa = self.inner_html_ele.select(&selector).next();

        if ipa.is_some() {
            let ipa_text = ipa.unwrap().text().next().unwrap();

            return Some(ipa_text.to_string());
        }

        return None;
    }

    fn get_us_ipa(&self) -> Option<String> {
        let selector = Selector::parse(".us.dpron-i .ipa.dipa.lpr-2.lpl-1").unwrap();
        let tag = self.inner_html_ele.select(&selector).next();
        if tag.is_some() {
            let ipa = tag.unwrap().text().next().unwrap();
            return Some(ipa.to_string());
        }

        None
    }
}

pub struct ClassDefinitionSection<'a> {
    pub inner_html_ele: ElementRef<'a>,
    word_class: &'a WordClassSection<'a>,
}

impl<'a> ClassDefinitionSection<'a> {
    pub fn new(inner_html_ele: ElementRef<'a>, word_class: &'a WordClassSection<'a>) -> Self {
        Self {
            inner_html_ele,
            word_class,
        }
    }

    pub fn get_definition(&self) -> ClassDefinition {
        ClassDefinition {
            contexts: self.contexts().iter().map(|x| x.get_context()).collect(),
        }
    }

    fn contexts(&'a self) -> Vec<ContextBlock<'a>> {
        let selector = Selector::parse(".pr.dsense").unwrap();
        let blocks = self
            .inner_html_ele
            .select(&selector)
            .map(|x| ContextBlock::new(x, &self))
            .collect();

        blocks
    }
}

pub struct ContextBlock<'a> {
    pub inner_html_ele: ElementRef<'a>,
    class_definition: &'a ClassDefinitionSection<'a>,
}

impl<'a> ContextBlock<'a> {
    pub fn new(
        inner_html_ele: ElementRef<'a>,
        class_definition: &'a ClassDefinitionSection<'a>,
    ) -> Self {
        Self {
            inner_html_ele,
            class_definition,
        }
    }

    pub fn get_context(&self) -> WordContext {
        WordContext {
            description: self.get_context_description(),
            meanings: self
                .word_meaning_blocks()
                .iter()
                .map(|x| x.get_meaning())
                .collect(),
        }
    }

    fn get_context_description(&self) -> Option<String> {
        let selector = Selector::parse(".guideword.dsense_gw span").unwrap();
        let sample = self.inner_html_ele.select(&selector).next();
        if sample.is_none() {
            return None;
        }

        let description = self
            .inner_html_ele
            .select(&selector)
            .next()
            .unwrap()
            .text()
            .next()
            .unwrap();

        Some(description.to_string())
    }

    fn word_meaning_blocks(&'a self) -> Vec<WordMeaningBlock<'a>> {
        let selector = Selector::parse(".def-block.ddef_block").unwrap();
        self.inner_html_ele
            .select(&selector)
            .map(|x| WordMeaningBlock::new(x, &self))
            .collect()
    }
}

struct WordMeaningBlock<'a> {
    inner_html_ele: ElementRef<'a>,
    context: &'a ContextBlock<'a>,
}

impl<'a> WordMeaningBlock<'a> {
    pub fn new(inner_html_ele: ElementRef<'a>, context: &'a ContextBlock<'a>) -> Self {
        Self {
            inner_html_ele,
            context,
        }
    }

    fn get_explanation(&self) -> String {
        let selector = Selector::parse(".def.ddef_d.db").unwrap();

        self.inner_html_ele
            .select(&selector)
            .next()
            .unwrap()
            .text()
            .fold("".to_string(), |mut acc, e| {
                acc.push_str(e);
                acc
            })
    }

    fn get_examples(&self) -> Vec<WordUsageExample> {
        let selector = Selector::parse(".examp.dexamp").unwrap();
        self.inner_html_ele
            .select(&selector)
            .map(|x| WordUsageExample {
                0: x.text().fold("".to_owned(), |mut acc, e| {
                    acc.push_str(e);
                    acc
                }),
            })
            .collect()
    }

    pub fn get_meaning(&self) -> WordExplanation {
        WordExplanation {
            explanation: self.get_explanation(),
            examples: self.get_examples(),
        }
    }
}
