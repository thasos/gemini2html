use log::{debug, error, info};
use std::fmt;
use std::fs::File;
use std::io::{Read, Write};
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
            // TODO quote here, no `<br />`
            ParsedGemini::Quote(quote) => format!("TODO QUOTE: {}<br />", quote),
            ParsedGemini::PreformattedStart => "<pre>".to_string(),
            ParsedGemini::PreformattedEnd => "</pre>".to_string(),
            ParsedGemini::PreformattedText(text) => escaped_preformat_text(text),
            ParsedGemini::Text(text) => format!("<p>{}</p>", text),
        }
    }
}

/// Return html headers, the title of the page should be known
fn html_headers(title: Option<&str>) -> String {
    let title = title.unwrap_or("some title");
    let css = html_style();
    format!(
        "<!doctype html>\n<html>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n{}\n<title>{}</title>\n<body>\n",
        css, title
    )
}

/// Read css file and put it in `style` tags
fn html_style() -> String {
    // let css_path = Path::new("style.css");
    // let css = fs::read_to_string(css_path).expect("unable to read file in ./output_tests");
    let css = include_bytes!("style.css");
    let css = match std::str::from_utf8(css) {
        Ok(css) => css.to_string(),
        Err(e) => {
            error!("unable to find css file {:?}", e);
            String::from("")
        }
    };
    format!("<style>\n{css}</style>")
}

/// Return html footer and closing tags, we can pass some infos here
/// like copyright, version, and link to project...
fn html_footers(infos: &str) -> String {
    format!("<p>{}</p></body>\n</html>\n", infos)
}

/// Read a line, an replace characters that must be escaped for preformatted html
fn escaped_preformat_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Format a gimini link [+ description] to a html link `<a>` tag,
/// if the link seems to point an image, format a `<img>` tag
fn html_link(link: &str) -> String {
    // trim start before split or description will be lost
    let link = link.trim_start();
    // if a description is present, use it in <a> tag
    let (url, description) = match link.split_once(' ') {
        Some(splitted_link) => splitted_link,
        // if no description is provided, use the link as text
        None => (link, link),
    };
    // handle image
    // TODO no `<br \>` here, use css...
    let image_html_options = r#"loading="lazy" height="200" sizes="auto, (max-width: 30em) 100vw, (max-width: 50em) 50vw, calc(33vw - 100px)""#;
    // let image_html_options = r#"loading="lazy" width="200" height="200" sizes="auto, (max-width: 30em) 100vw, (max-width: 50em) 50vw, calc(33vw - 100px)""#;
    let image_format = format!(
        "<a href=\"{url}\"><img {image_html_options} src=\"{url}\" alt=\"{description}\" /></a><br />"
    );
    let standard_link_format = format!("<a href=\"{url}\">{description}</a><br />");
    // try to match a known image extension
    // TODO downcase
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
            "avif" => image_format,
            // unknown extension : standard link
            _ => standard_link_format,
        },
        // unable to find an extension : standard link
        None => standard_link_format,
    }
}

/// Standard read file
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

/// Standard write to file
fn write_to_file(path: &Path, content: &str) -> Result<()> {
    let mut file = File::create(path).map_err(|e| {
        error!("unable to open file {} : {e:?}", path.to_string_lossy());
        Gemini2HtmlError
    })?;
    // TODO handle result
    file.write_all(content.as_bytes()).map_err(|e| {
        error!("unable to write to file {} : {e:?}", path.to_string_lossy());
        Gemini2HtmlError
    })?;
    Ok(())
}

/// Main course here, read a gemini content line by line
/// if a syntax element is found, store the line in the corresponding enum
/// for lists and preformatted text, use flags for beginning and end tags
fn parse_gemini(gemini_content: &str) -> (Vec<ParsedGemini>, Option<&str>) {
    // init the Vec returned
    let mut parsed_gemini: Vec<ParsedGemini> = Vec::new();
    let mut title: Option<&str> = None;
    // create flags for listes and preformatted text
    let mut flag_list = false;
    let mut flag_preformatted = false;
    for line in gemini_content.lines() {
        // in case some spaces are present before syntax elements
        let line = line.trim_start();
        // if the line contain only preformatted tag, we don't need to go further
        if line == "```" {
            // not already preformatted ? start !
            if !flag_preformatted {
                parsed_gemini.push(ParsedGemini::PreformattedStart);
                flag_preformatted = true;
            // another `<pre>` ? end it...
            } else {
                parsed_gemini.push(ParsedGemini::PreformattedEnd);
                flag_preformatted = false;
            }
        // if we are in a preformatted block, we need to escape reserved chars
        // see https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/pre
        // `&` -> `&amp;`
        // `<` and `>` -> `&lt;` and `&gt;`
        } else if flag_preformatted {
            parsed_gemini.push(ParsedGemini::PreformattedText(line.to_string()));
        } else {
            // split line in two, matching the first space
            let (line_prefix, line_rest) = line.split_once(' ').unwrap_or_default();

            // if we were in a list, but not anymore, end it
            if flag_list && line_prefix != "*" {
                parsed_gemini.push(ParsedGemini::ListEnd);
                flag_list = false;
            }

            // now we search if the first word match a syntax, and push the rest in output Vec
            match line_prefix {
                "=>" => parsed_gemini.push(ParsedGemini::Link(line_rest.to_string())),
                "#" => {
                    title = Some(line_rest);
                    parsed_gemini.push(ParsedGemini::Heading1(line_rest.to_string()))
                }
                "##" => parsed_gemini.push(ParsedGemini::Heading2(line_rest.to_string())),
                "###" => parsed_gemini.push(ParsedGemini::Heading3(line_rest.to_string())),
                // a list must begin with tag `<ul>` and start with `</ul>`
                "*" => {
                    // begin the list
                    if !flag_list {
                        parsed_gemini.push(ParsedGemini::ListStart);
                        flag_list = true;
                    }
                    parsed_gemini.push(ParsedGemini::ListElement(line_rest.to_string()));
                }
                ">" => parsed_gemini.push(ParsedGemini::Quote(line_rest.to_string())),
                // for text, we need all the line
                _ => parsed_gemini.push(ParsedGemini::Text(line.to_string())),
            }
        }
    }
    (parsed_gemini, title)
}

/// Eat parsed gemini Vec, and create a formatted html page
fn format_gemini_to_html(parsed_gemini: Vec<ParsedGemini>, title: Option<&str>) -> String {
    let mut html_content = String::new();
    // init html main tags
    let headers = html_headers(title);
    let footers = html_footers("some infos");
    // let's construct html document
    html_content.push_str(&headers);
    // insert gemini element formatted to html
    for line in parsed_gemini {
        html_content.push_str(&line.to_html());
        // insert line feed between each elemets
        html_content.push('\n');
    }
    html_content.push_str(&footers);
    html_content
}

/// Read file, pass content to the parser, and write the output to the target file
pub fn convert_gemini_file(gemini_file_path: &Path, target_file: &Path) -> Result<()> {
    match read_from_file(gemini_file_path) {
        Ok(gemini_file_content) => {
            info!("    - 🟢 start file {:?}", gemini_file_path);
            let (parsed_gemini, title) = parse_gemini(&gemini_file_content);
            info!("    - 🍽️  parsed");
            let html_content = format_gemini_to_html(parsed_gemini, title);
            info!("    - 🎨 converted to html");
            write_to_file(target_file, &html_content)?;
            info!("    - 🪦 html saved to file {:?}", target_file);
        }
        Err(e) => error!("{}", e),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_html_headers_and_footers() {
        let footers = html_footers("some footers");
        assert_eq!(footers, "<p>some footers</p></body>\n</html>\n");
        let headers = html_headers(Some("A cool title 🪻"));
        assert_eq!(
            headers,
            "<!doctype html>\n<html>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n<style>\nhtml {\n\tfont-family: sans-serif;\n\tcolor: #080808;\n}\n\nbody {\n\tmax-width: 920px;\n\tmargin: 0 auto;\n\tpadding: 1rem 2rem;\n}\n\nblockquote {\n\tbackground-color: #eee;\n\tborder-left: 3px solid #444;\n\tmargin: 1rem -1rem 1rem calc(-1rem - 3px);\n\tpadding: 1rem;\n}\n\nul {\n\tmargin-left: 0;\n\tpadding: 0;\n}\n\nli {\n\tpadding: 0;\n}\n\nli:not(:last-child) {\n\tmargin-bottom: 0.5rem;\n}\n\na {\n\tposition: relative;\n}\n\na:before {\n\tcontent: '⇒';\n\tcolor: #999;\n\ttext-decoration: none;\n\tfont-weight: bold;\n\tposition: absolute;\n\tleft: -1.25rem;\n}\n\npre {\n\tbackground-color: #eee;\n\tmargin: 0 -1rem;\n\tpadding: 1rem;\n\toverflow-x: auto;\n}\n\ndetails:not([open]) summary,\ndetails:not([open]) summary a {\n\tcolor: gray;\n}\n\ndetails summary a:before {\n\tdisplay: none;\n}\n\ndl dt {\n\tfont-weight: bold;\n}\n\ndl dt:not(:first-child) {\n\tmargin-top: 0.5rem;\n}\n\n@media(prefers-color-scheme:dark) {\n\thtml {\n\t\tbackground-color: #111;\n\t\tcolor: #eee;\n\t}\n\n\tblockquote {\n\t\tbackground-color: #000;\n\t}\n\n\tpre {\n\t\tbackground-color: #222;\n\t}\n\n\ta {\n\t\tcolor: #0087BD;\n\t}\n\n\ta:visited {\n\t\tcolor: #333399;\n\t}\n}\n\nlabel {\n\tdisplay: block;\n\tfont-weight: bold;\n\tmargin-bottom: 0.5rem;\n}\n\ninput {\n\tdisplay: block;\n\tborder: 1px solid #888;\n\tpadding: .375rem;\n\tline-height: 1.25rem;\n\ttransition: border-color .15s ease-in-out,box-shadow .15s ease-in-out;\n\twidth: 100%;\n}\n\ninput:focus {\n\toutline: 0;\n\tborder-color: #80bdff;\n\tbox-shadow: 0 0 0 0.2rem rgba(0,123,255,.25);\n}\n</style>\n<title>A cool title 🪻</title>\n<body>\n"
        );
    }
    #[test]
    fn test_escaped_preformat_text() {
        let a_complicated_line = "&;\"🌳";
        let escaped_line = escaped_preformat_text(a_complicated_line);
        assert_eq!(escaped_line, "&amp;;\"🌳".to_string());
    }
    #[test]
    fn test_write_and_read_file() {
        let content = "some content";
        let target_path = Path::new("./tests/target_file.test");
        // write file
        let write_result = write_to_file(target_path, content);
        assert_eq!(write_result, Ok(()));
        // read file
        let file_content = read_from_file(Path::new("./tests/target_file.test")).unwrap();
        assert_eq!(file_content, "some content");
        // clean test file
        fs::remove_file(target_path)
            .expect("unable to delete test file (./tests/target_file.test)");
    }
    #[test]
    fn test_format_gemini_to_html() {
        let (parsed_gemini, title) =
            parse_gemini("## heading2\n* tiny list\n```\npreformatted &text\n```\n");
        let html_content = format_gemini_to_html(parsed_gemini, title);
        assert_eq!(
            html_content,
            "<!doctype html>\n<html>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n<style>\nhtml {\n\tfont-family: sans-serif;\n\tcolor: #080808;\n}\n\nbody {\n\tmax-width: 920px;\n\tmargin: 0 auto;\n\tpadding: 1rem 2rem;\n}\n\nblockquote {\n\tbackground-color: #eee;\n\tborder-left: 3px solid #444;\n\tmargin: 1rem -1rem 1rem calc(-1rem - 3px);\n\tpadding: 1rem;\n}\n\nul {\n\tmargin-left: 0;\n\tpadding: 0;\n}\n\nli {\n\tpadding: 0;\n}\n\nli:not(:last-child) {\n\tmargin-bottom: 0.5rem;\n}\n\na {\n\tposition: relative;\n}\n\na:before {\n\tcontent: '⇒';\n\tcolor: #999;\n\ttext-decoration: none;\n\tfont-weight: bold;\n\tposition: absolute;\n\tleft: -1.25rem;\n}\n\npre {\n\tbackground-color: #eee;\n\tmargin: 0 -1rem;\n\tpadding: 1rem;\n\toverflow-x: auto;\n}\n\ndetails:not([open]) summary,\ndetails:not([open]) summary a {\n\tcolor: gray;\n}\n\ndetails summary a:before {\n\tdisplay: none;\n}\n\ndl dt {\n\tfont-weight: bold;\n}\n\ndl dt:not(:first-child) {\n\tmargin-top: 0.5rem;\n}\n\n@media(prefers-color-scheme:dark) {\n\thtml {\n\t\tbackground-color: #111;\n\t\tcolor: #eee;\n\t}\n\n\tblockquote {\n\t\tbackground-color: #000;\n\t}\n\n\tpre {\n\t\tbackground-color: #222;\n\t}\n\n\ta {\n\t\tcolor: #0087BD;\n\t}\n\n\ta:visited {\n\t\tcolor: #333399;\n\t}\n}\n\nlabel {\n\tdisplay: block;\n\tfont-weight: bold;\n\tmargin-bottom: 0.5rem;\n}\n\ninput {\n\tdisplay: block;\n\tborder: 1px solid #888;\n\tpadding: .375rem;\n\tline-height: 1.25rem;\n\ttransition: border-color .15s ease-in-out,box-shadow .15s ease-in-out;\n\twidth: 100%;\n}\n\ninput:focus {\n\toutline: 0;\n\tborder-color: #80bdff;\n\tbox-shadow: 0 0 0 0.2rem rgba(0,123,255,.25);\n}\n</style>\n<title>some title</title>\n<body>\n<h2>heading2</h2>\n<ul>\n<li>tiny list</li>\n<pre>\npreformatted &amp;text\n</pre>\n<p>some infos</p></body>\n</html>\n"
        );
    }
    #[test]
    fn test_html_link() {
        // simple
        let simple_link = "protocol://fqdn/path";
        let htmled_link = html_link(simple_link);
        assert_eq!(
            htmled_link,
            "<a href=\"protocol://fqdn/path\">protocol://fqdn/path</a><br />".to_string()
        );
        // description
        let simple_link_with_description = "protocol://fqdn/path some nice description";
        let htmled_link = html_link(simple_link_with_description);
        assert_eq!(
            htmled_link,
            "<a href=\"protocol://fqdn/path\">some nice description</a><br />".to_string()
        );
        // image
        let simple_link_to_image = "protocol://fqdn/path.png";
        let htmled_link = html_link(simple_link_to_image);
        assert_eq!(
            htmled_link,
            "<a href=\"protocol://fqdn/path.png\"><img loading=\"lazy\" height=\"200\" sizes=\"auto, (max-width: 30em) 100vw, (max-width: 50em) 50vw, calc(33vw - 100px)\" src=\"protocol://fqdn/path.png\" alt=\"protocol://fqdn/path.png\" /></a><br />".to_string()
        );
        // image with description
        let simple_link_to_image_with_description =
            "protocol://fqdn/path.png some nice image description";
        let htmled_link = html_link(simple_link_to_image_with_description);
        assert_eq!(
            htmled_link,
            "<a href=\"protocol://fqdn/path.png\"><img loading=\"lazy\" height=\"200\" sizes=\"auto, (max-width: 30em) 100vw, (max-width: 50em) 50vw, calc(33vw - 100px)\" src=\"protocol://fqdn/path.png\" alt=\"some nice image description\" /></a><br />".to_string()
        );
        // first characters are spaces
        let simple_link_with_spaces = "    protocol://fqdn/path";
        let htmled_link = html_link(simple_link_with_spaces);
        assert_eq!(
            htmled_link,
            "<a href=\"protocol://fqdn/path\">protocol://fqdn/path</a><br />".to_string()
        );
    }
}
