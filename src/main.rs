use chrono::{Duration, Local};
use std::collections::BTreeMap;
use std::collections::HashMap;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Cli {
    #[structopt(short = "a", long = "area", default_value = "東京")]
    area: String,
}

#[derive(Debug)]
#[allow(dead_code)]
struct Program {
    id: String,
    channel: usize,
    name: String,
    description: String,
    link: String,
    start_time: String,
    end_time: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::from_args();

    let area_ids: HashMap<&str, &str> = [
        ("札幌", "1"),
        ("函館", "8"),
        ("旭川", "3"),
        ("帯広", "9"),
        ("釧路", "10"),
        ("北見", "12"),
        ("室蘭", "6"),
        ("青森", "13"),
        ("岩手", "16"),
        ("宮城", "19"),
        ("秋田", "22"),
        ("山形", "25"),
        ("福島", "28"),
        ("東京", "42"),
        ("神奈川", "45"),
        ("埼玉", "37"),
        ("千葉", "40"),
        ("茨城", "31"),
        ("栃木", "33"),
        ("群馬", "35"),
        ("山梨", "50"),
        ("長野", "51"),
        ("新潟", "56"),
        ("愛知", "73"),
        ("石川", "60"),
        ("静岡", "67"),
        ("福井", "62"),
        ("富山", "58"),
        ("三重", "76"),
        ("岐阜", "64"),
        ("大阪", "84"),
        ("京都", "81"),
        ("兵庫", "85"),
        ("和歌山", "93"),
        ("奈良", "91"),
        ("滋賀", "79"),
        ("広島", "101"),
        ("岡山", "98"),
        ("島根", "96"),
        ("鳥取", "95"),
        ("山口", "105"),
        ("愛媛", "112"),
        ("香川", "110"),
        ("徳島", "109"),
        ("高知", "116"),
        ("福岡", "117"),
        ("熊本", "126"),
        ("長崎", "123"),
        ("鹿児島", "131"),
        ("宮崎", "129"),
        ("大分", "127"),
        ("佐賀", "122"),
        ("沖縄", "134"),
        ("北九州", "120"),
    ]
    .iter()
    .cloned()
    .collect();

    let area_id = area_ids.get(&*args.area.to_string());
    if area_id.is_none() {
        let mut s = String::new();
        for k in area_ids.keys() {
            if s.is_empty() {
                s = k.to_string();
            } else {
                s = format!("{} {}", s, k);
            }
        }
        panic!("invalid area, please choose from here.\n{}", s);
    }

    let now = Local::now().format("%Y%m%d%H%M").to_string();
    let date = &now[0..8];

    let url = format!(
        "https://bangumi.org/epg/td?broad_cast_date={}&ggm_group_id={}",
        date,
        area_id.unwrap()
    );

    let html = reqwest::get(url).await?.text().await?;
    let document = scraper::Html::parse_document(&html);
    let channels: Vec<String> = get_channels(&document);
    let program_selector = scraper::Selector::parse("div #program_area ul li").unwrap();

    let mut programs = BTreeMap::new();
    for node in document.select(&program_selector) {
        let parent = node.parent();
        let channel = parent
            .iter()
            .next()
            .unwrap()
            .value()
            .as_element()
            .unwrap()
            .id()
            .unwrap()
            .to_string();
        let channel_id: usize = channel
            .replace("program_line_", "")
            .parse()
            .unwrap();

        let id = node.value().attr("se-id");
        let inner_html = node.inner_html();
        let fragment = scraper::Html::parse_fragment(&inner_html);
        let name = get_program_name(&fragment);
        let link = get_program_link(&fragment);
        let description = get_program_description(&fragment);
        let start_time = node.value().attr("s");
        let end_time = node.value().attr("e");

        if name.is_empty() {
            continue;
        }

        programs.insert(
            start_time.unwrap_or("").to_string() + "_" + &channel_id.to_string(),
            Program {
                id: id.unwrap_or("").to_string()[7..].to_string(),
                channel: channel_id,
                name,
                description,
                link,
                start_time: start_time.unwrap_or("").to_string(),
                end_time: end_time.unwrap_or("").to_string(),
            },
        );
    }

    let target = (Local::now() + Duration::hours(2)).format("%Y%m%d%H%M").to_string();
    let mut lines: HashMap<&str, bool> = HashMap::new();
    for v in programs.values() {
        if v.end_time < now || v.end_time > target {
            continue;
        }
        let month: String = (&v.start_time)[4..6].to_string();
        let day: String = (&v.start_time)[6..8].to_string();
        let hour: String = (&v.start_time)[8..10].to_string();

        let k: String = (&v.start_time)[0..10].to_string();
        if !lines.contains_key(&*k) {
            println!(
                "\n{}/{} {}時~\n--------------------------------------------",
                month, day, hour
            );
            lines.insert(&v.start_time[0..10], true);
        }

        println!(
            "{}:{}~{}:{} 【{}】\t{} [{}]",
            &v.start_time[8..10],
            &v.start_time[10..12],
            &v.end_time[8..10],
            &v.end_time[10..12],
            channels[v.channel - 1],
            v.name,
            v.id,
        );
    }

    println!("\nThis TV schedule is got from テレビ番組表Gガイド(https://bangumi.org).");

    Ok(())
}

fn get_channels(document: &scraper::Html) -> Vec<String> {
    let mut channels: Vec<String> = Vec::new();
    let channel_selector = scraper::Selector::parse("div #ch_area ul li p").unwrap();
    for node in document.select(&channel_selector) {
        channels.push(node.text().next().unwrap_or("").to_string());
    }

    channels
}

fn get_program_description(document: &scraper::Html) -> String {
    let selector = scraper::Selector::parse("div p.program_detail").unwrap();
    document
        .select(&selector)
        .next()
        .and_then(|p| p.text().next())
        .unwrap_or("")
        .to_string()
}

fn get_program_link(document: &scraper::Html) -> String {
    let selector = scraper::Selector::parse("div a").unwrap();
    document
        .select(&selector)
        .next()
        .and_then(|p| p.value().attr("href"))
        .unwrap_or("")
        .to_string()
}

fn get_program_name(document: &scraper::Html) -> String {
    let selector = scraper::Selector::parse(".program_title").unwrap();
    document
        .select(&selector)
        .next()
        .and_then(|p| p.text().next())
        .unwrap_or("")
        .to_string()
}
