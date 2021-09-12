use chrono::Local;
use scraper;
use std::collections::BTreeMap;
use std::collections::HashMap;

#[derive(Debug)]
struct Program {
    id: String,
    name: String,
    description: String,
    link: String,
    start_time: String,
    end_time: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let area_id = area_ids.get("東京");
    if area_id.is_none() {
        panic!("invalid area");
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
    let selector = scraper::Selector::parse("div #program_area ul li").unwrap();

    let mut programs = BTreeMap::new();
    for node in document.select(&selector) {
        let id = node.value().attr("se-id");

        let inner_html = node.inner_html();
        let fragment = scraper::Html::parse_fragment(&inner_html);

        let name_selector = scraper::Selector::parse("div a p").unwrap();
        let mut name = None;
        for p in fragment.select(&name_selector) {
            name = p.text().next();
            break;
        }

        let link_selector = scraper::Selector::parse("div a").unwrap();
        let mut link = None;
        for p in fragment.select(&link_selector) {
            link = p.value().attr("href");
            break;
        }

        let description_selector = scraper::Selector::parse("div p.program_detail").unwrap();
        let mut description = None;
        for p in fragment.select(&description_selector) {
            description = p.text().next();
            break;
        }

        let start_time = node.value().attr("s");
        let end_time = node.value().attr("e");

        if name.is_none() {
            continue;
        }
        programs.insert(
            start_time.unwrap_or("").to_string(),
            Program {
                id: id.unwrap_or("").to_string()[7..].to_string(),
                name: name.unwrap_or("").to_string(),
                description: description.unwrap_or("").to_string(),
                link: link.unwrap_or("").to_string(),
                start_time: start_time.unwrap_or("").to_string(),
                end_time: end_time.unwrap_or("").to_string(),
            },
        );
    }

    for (_k, v) in &programs {
        if v.end_time < now {
            continue;
        }

        println!(
            "{}:{}~{}:{} {} [{}]",
            &v.start_time[8..10],
            &v.start_time[10..12],
            &v.end_time[8..10],
            &v.end_time[10..12],
            v.name,
            v.id,
        );
    }

    println!("\nThis TV schedule is got from テレビ番組表Gガイド(https://bangumi.org)");

    Ok(())
}
