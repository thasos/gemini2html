use log::{debug, error, info};
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Error handling, I should rework this awfull part...
pub type Result<T> = std::result::Result<T, Gemini2HtmlError>;
#[derive(Debug, Clone, PartialEq)]
pub struct Gemini2HtmlError;
impl fmt::Display for Gemini2HtmlError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "😢 Sorry, gemini2html encountered an error.")
    }
}

/// Gemini syntax elements
/// see https://geminiprotocol.net/docs/gemtext-specification.gmi for details
enum ParsedGemini {
    Link(String),
    Heading1(String),
    Heading2(String),
    Heading3(String),
    ListStart,
    ListEnd,
    ListElement(String),
    Quote(String),
    Text(String),
    PreformattedStart,
    PreformattedEnd,
    PreformattedText(String),
}
impl ParsedGemini {
    /// format gemini elements to html
    fn to_html(&self) -> String {
        match self {
            ParsedGemini::Link(link) => html_link(link),
            ParsedGemini::Heading1(heading) => format!("<h1>{}</h1>", heading),
            ParsedGemini::Heading2(heading) => format!("<h2>{}</h2>", heading),
            ParsedGemini::Heading3(heading) => format!("<h3>{}</h3>", heading),
            ParsedGemini::ListStart => "<ul>".to_string(),
            ParsedGemini::ListEnd => "</ul>".to_string(),
            ParsedGemini::ListElement(list) => format!("<li>{}</li>", list),
            ParsedGemini::Quote(quote) => format!("TODO QUOTE{}", quote),
            ParsedGemini::PreformattedStart => "<pre>".to_string(),
            ParsedGemini::PreformattedEnd => "</pre>".to_string(),
            ParsedGemini::PreformattedText(text) => escaped_preformat_text(text),
            ParsedGemini::Text(text) => format!("<p>{}</p>", text),
        }
    }
}

/// return html headers, the title of the page should be known
fn html_headers(title: Option<&str>) -> String {
    match title {
        Some(title) =>
    format!(
        "<!doctype html>\n<html>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n
    <title>{}</title>\n<body>", title),
        None => "some title".to_string(),
    }
}

/// return html footer and closing tags, we can pass some infos here
/// like copyright, version, and link to project...
fn html_footers(infos: &str) -> String {
    format!("<p>{}</p></body>\n</html>", infos)
}

/// read a line, an replace characters that must be escaped for preformatted html
fn escaped_preformat_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// format a gimini link [+ description] to a html link `<a>` tag,
/// if the link seems to point an image, format a `<img>` tag
fn html_link(link: &str) -> String {
    // if a description is present, use it in <a> tag
    let (url, description) = match link.split_once(' ') {
        Some(splitted_link) => splitted_link,
        // if no description is provided, use the link as text
        None => (link, link),
    };

    // handle image
    let image_format = format!("<a href=\"{url}\"><img src=\"{url}\" alt=\"{description}\" /></a>");
    let standard_link_format = format!("<a href=\"{url}\">{description}</a>");
    // try to match a known image extension
    match url.rsplit_once('.') {
        Some((_reste, extension)) => match extension {
            "jpg" => image_format,
            "png" => image_format,
            "gif" => image_format,
            "webp" => image_format,
            "tiff" => image_format,
            "bmp" => image_format,
            "jpeg" => image_format,
            "svg" => image_format,
            // unknown extension : standard link
            _ => standard_link_format,
        },
        // unable to find an extension : standard link
        None => standard_link_format,
    }
}

/// standard read
fn read_from_file(path: &Path) -> Result<String> {
    debug!("💨 open file {:?}", path);
    let mut file = File::open(path).map_err(|e| {
        error!("unable to open file {} : {e:?}", path.to_string_lossy());
        Gemini2HtmlError
    })?;
    let mut file_content = String::new();
    file.read_to_string(&mut file_content).map_err(|e| {
        error!(
            "unable to convert file {} to string : {e:?}",
            path.to_string_lossy()
        );
        Gemini2HtmlError
    })?;
    Ok(file_content)
}

/// main course here, read a gemini content line by line
/// if a syntax element is found, store the line in the corresponding enum
/// for lists and preformatted text, use flags for beginning and end tags
fn parse_gemini_and_format_html(input_gemini: &str) {
    // init the Vec returned
    let mut output_parsed_text: Vec<ParsedGemini> = Vec::new();
    // create flags for listes and preformatted text
    let mut flag_list = false;
    let mut flag_preformatted = false;
    for line in input_gemini.lines() {
        // in case some spaces are present before syntax elements
        let line = line.trim_start();
        // if the line contain only preformatted tag, we don't need to go further
        if line == "```" {
            // not already preformatted ? start !
            if !flag_preformatted {
                output_parsed_text.push(ParsedGemini::PreformattedStart);
                flag_preformatted = true;
            // another `<pre>` ? end it...
            } else {
                output_parsed_text.push(ParsedGemini::PreformattedEnd);
                flag_preformatted = false;
            }
        // if we are in a preformatted block, we need to escape reserved chars
        // see https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/pre
        // `&` -> `&amp;`
        // `<` and `>` -> `&lt;` and `&gt;`
        } else if flag_preformatted {
            output_parsed_text.push(ParsedGemini::PreformattedText(line.to_string()));
        } else {
            // split line in two, matching the first space
            let (line_prefix, line_rest) = line.split_once(' ').unwrap_or_default();

            // if we were in a list, but not anymore, end it
            if flag_list && line_prefix != "*" {
                output_parsed_text.push(ParsedGemini::ListEnd);
                flag_list = false;
            }

            // now we search if the first word match a syntax, and push the rest in output Vec
            match line_prefix {
                "=>" => output_parsed_text.push(ParsedGemini::Link(line_rest.to_string())),
                "#" => output_parsed_text.push(ParsedGemini::Heading1(line_rest.to_string())),
                "##" => output_parsed_text.push(ParsedGemini::Heading2(line_rest.to_string())),
                "###" => output_parsed_text.push(ParsedGemini::Heading3(line_rest.to_string())),
                // a list must begin with tag `<ul>` and start with `</ul>`
                "*" => {
                    // begin the list
                    if !flag_list {
                        output_parsed_text.push(ParsedGemini::ListStart);
                        flag_list = true;
                    }
                    output_parsed_text.push(ParsedGemini::ListElement(line_rest.to_string()));
                }
                ">" => output_parsed_text.push(ParsedGemini::Quote(line_rest.to_string())),
                // for text, we need all the line
                _ => output_parsed_text.push(ParsedGemini::Text(line.to_string())),
            }
        }
    }
    // TODO virer ça
    let headers = html_headers(None);
    let footers = html_footers("some infos");
    println!("{headers}");
    for i in output_parsed_text {
        println!("{}", i.to_html());
    }
    println!("{footers}");
}

/// read file, pass content to the parser, and write the output to the target file
// TODO write to file
pub fn parse_gemini_file(gemini_file_path: &Path) {
    match read_from_file(gemini_file_path) {
        Ok(gemini_file_content) => {
            info!("🍽️  start parse file {:?}", gemini_file_path);
            parse_gemini_and_format_html(&gemini_file_content);
            info!("🏁 end parse file {:?}", gemini_file_path);
        }
        Err(e) => error!("{}", e),
    }
}

// TODO later....
// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn it_works() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }
