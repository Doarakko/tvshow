use chrono::{Duration, Local, NaiveDateTime, Timelike};
use std::collections::BTreeMap;
use std::collections::HashMap;
use structopt::StructOpt;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(StructOpt)]
struct Cli {
    #[structopt(short = "a", long = "area", default_value = "東京")]
    area: String,
    #[structopt(short = "t", long = "hours", default_value = "12")]
    hours: i64,
}

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
struct Dates {
    date: String,
    next_date: String,
    needs_next_day: bool,
}

const AREA_IDS: &[(&str, &str)] = &[
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
];

const COL_WIDTH: usize = 20;
const TIME_WIDTH: usize = 11;
const MAX_CHANNELS: usize = 8;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::from_args();

    let area_id = match resolve_area_id(&args.area) {
        Some(id) => id,
        None => panic!("invalid area, please choose from here.\n{}", area_names()),
    };

    let now = Local::now().naive_local();
    let now_str = now.format("%Y%m%d%H%M").to_string();
    let dates = resolve_dates(now, args.hours);

    let mut programs = BTreeMap::new();
    let program_selector = scraper::Selector::parse("div #program_area ul li").unwrap();

    // 1日目のデータを取得
    let url = epg_url(&dates.date, area_id);
    let html = reqwest::get(&url).await?.text().await?;
    let document = scraper::Html::parse_document(&html);
    let channels: Vec<String> = get_channels(&document);
    parse_programs(&document, &program_selector, &mut programs);

    // 2日目のデータを取得（必要な場合）
    if dates.needs_next_day {
        let url2 = epg_url(&dates.next_date, area_id);
        if let Ok(html2) = reqwest::get(&url2).await {
            if let Ok(text) = html2.text().await {
                let document2 = scraper::Html::parse_document(&text);
                parse_programs(&document2, &program_selector, &mut programs);
            }
        }
    }

    let target = (now + Duration::hours(args.hours))
        .format("%Y%m%d%H%M")
        .to_string();

    // チャンネルごとに番組をグループ化
    let channel_programs = filter_programs(&programs, &now_str, &target);
    // 存在するチャンネルをソートして最大8チャンネルまで表示
    let sorted_channels = visible_channels(&channel_programs);

    print!(
        "{}",
        render_schedule(
            &channels,
            &channel_programs,
            &sorted_channels,
            now.hour() as i32,
            args.hours,
        )
    );

    println!("\nThis TV schedule is got from テレビ番組表Gガイド(https://bangumi.org).");

    Ok(())
}

fn epg_url(date: &str, area_id: &str) -> String {
    format!(
        "https://bangumi.org/epg/td?broad_cast_date={}&ggm_group_id={}",
        date, area_id
    )
}

fn resolve_area_id(area: &str) -> Option<&'static str> {
    AREA_IDS
        .iter()
        .find(|(name, _)| *name == area)
        .map(|(_, id)| *id)
}

fn area_names() -> String {
    AREA_IDS
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<&str>>()
        .join(" ")
}

fn resolve_dates(now: NaiveDateTime, hours: i64) -> Dates {
    // TV番組表は5時を起点とするため、5時より前は前日の番組表を取得
    let date = if now.hour() < 5 {
        (now - Duration::days(1)).format("%Y%m%d").to_string()
    } else {
        now.format("%Y%m%d").to_string()
    };

    // 表示終了時刻が5時をまたぐ場合は翌日のデータも必要
    let target_time = now + Duration::hours(hours);
    let needs_next_day = if now.hour() < 5 {
        // 現在5時前の場合、表示範囲が5時以降にかかるなら当日のデータも必要
        target_time.hour() >= 5 || target_time.date() > now.date()
    } else {
        // 現在5時以降の場合、表示範囲が翌日5時以降にかかるなら翌日のデータも必要
        target_time.date() > now.date() && target_time.hour() >= 5
    };

    let next_date = if now.hour() < 5 {
        now.format("%Y%m%d").to_string()
    } else {
        (now + Duration::days(1)).format("%Y%m%d").to_string()
    };

    Dates {
        date,
        next_date,
        needs_next_day,
    }
}

fn filter_programs<'a>(
    programs: &'a BTreeMap<String, Program>,
    now_str: &str,
    target: &str,
) -> HashMap<usize, Vec<&'a Program>> {
    let mut channel_programs: HashMap<usize, Vec<&Program>> = HashMap::new();
    for v in programs.values() {
        // 終了済みの番組はスキップ
        if v.end_time.as_str() < now_str {
            continue;
        }
        // N時間以内に開始する番組のみ表示
        if v.start_time.as_str() > target {
            continue;
        }
        channel_programs.entry(v.channel).or_default().push(v);
    }

    channel_programs
}

fn visible_channels(channel_programs: &HashMap<usize, Vec<&Program>>) -> Vec<usize> {
    let mut sorted_channels: Vec<usize> = channel_programs.keys().cloned().collect();
    sorted_channels.sort();
    sorted_channels.truncate(MAX_CHANNELS);

    sorted_channels
}

fn render_schedule(
    channels: &[String],
    channel_programs: &HashMap<usize, Vec<&Program>>,
    sorted_channels: &[usize],
    current_hour: i32,
    hours: i64,
) -> String {
    let mut out = String::new();

    // ヘッダー出力（チャンネル名）
    out.push_str(&" ".repeat(TIME_WIDTH));
    for &ch in sorted_channels {
        let ch_name = if ch > 0 && ch <= channels.len() {
            truncate_string(&channels[ch - 1], COL_WIDTH - 1)
        } else {
            format!("Ch{}", ch)
        };
        out.push('│');
        out.push_str(&center_string(&ch_name, COL_WIDTH));
    }
    out.push_str("│\n");

    // 区切り線
    out.push_str(&"─".repeat(TIME_WIDTH));
    for _ in sorted_channels {
        out.push('┼');
        out.push_str(&"─".repeat(COL_WIDTH));
    }
    out.push_str("┤\n");

    // 現在時刻の時間から表示
    let end_hour = current_hour + hours as i32;

    for hour in current_hour..=end_hour {
        let display_hour = hour % 24;
        let hour_str = format!("{:02}", display_hour);

        // 各チャンネルのこの時間帯の番組を取得
        let mut hour_programs: Vec<Option<&Program>> = Vec::new();
        for &ch in sorted_channels {
            if let Some(progs) = channel_programs.get(&ch) {
                let prog = progs
                    .iter()
                    .find(|p| p.start_time.get(8..10) == Some(hour_str.as_str()));
                hour_programs.push(prog.copied());
            } else {
                hour_programs.push(None);
            }
        }

        // 1行目: 時刻と番組名
        out.push_str(&format!(" {:02}:00     ", display_hour));
        for prog in &hour_programs {
            out.push('│');
            if let Some(p) = prog {
                // 8..10 は find の述語で Some が保証済み。10..12 は12桁でない異常データ向け
                let time_str = format!(
                    "{}:{}",
                    p.start_time.get(8..10).unwrap_or("??"),
                    p.start_time.get(10..12).unwrap_or("??")
                );
                let name = truncate_string(&p.name, COL_WIDTH - 7);
                let cell = format!("{} {}", time_str, name);
                out.push_str(&pad_string(&cell, COL_WIDTH));
            } else {
                out.push_str(&" ".repeat(COL_WIDTH));
            }
        }
        out.push_str("│\n");

        // 2行目: 番組名の続き
        out.push_str(&" ".repeat(TIME_WIDTH));
        for prog in &hour_programs {
            out.push('│');
            if let Some(p) = prog {
                // 1行目で表示した分をスキップして続きを表示
                let name_chars: Vec<char> = strip_emoji(&p.name).chars().collect();
                let first_line_len = truncate_string(&p.name, COL_WIDTH - 7).chars().count();
                let remaining: String = name_chars.iter().skip(first_line_len).collect();
                let second_line = truncate_string(&remaining, COL_WIDTH - 1);
                out.push(' ');
                out.push_str(&pad_string(&second_line, COL_WIDTH - 1));
            } else {
                out.push_str(&" ".repeat(COL_WIDTH));
            }
        }
        out.push_str("│\n");
    }

    out
}

fn get_channels(document: &scraper::Html) -> Vec<String> {
    let mut channels: Vec<String> = Vec::new();
    let channel_selector = scraper::Selector::parse("div #ch_area ul li p").unwrap();
    for node in document.select(&channel_selector) {
        channels.push(node.text().next().unwrap_or("").to_string());
    }

    channels
}

fn parse_programs(
    document: &scraper::Html,
    program_selector: &scraper::Selector,
    programs: &mut BTreeMap<String, Program>,
) {
    for node in document.select(program_selector) {
        // 親要素が program_line_<数字> でない行は読み飛ばす
        let Some(channel_id) = node
            .parent()
            .and_then(|parent| parent.value().as_element().and_then(|e| e.id()))
            .and_then(|line_id| line_id.strip_prefix("program_line_"))
            .and_then(|channel| channel.parse::<usize>().ok())
        else {
            continue;
        };

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

        let key = start_time.unwrap_or("").to_string() + "_" + &channel_id.to_string();
        // 重複を避ける（同じ番組は上書きしない）
        programs.entry(key).or_insert_with(|| Program {
            id: id
                .unwrap_or("")
                .to_string()
                .get(7..)
                .unwrap_or("")
                .to_string(),
            channel: channel_id,
            name,
            description,
            link,
            start_time: start_time.unwrap_or("").to_string(),
            end_time: end_time.unwrap_or("").to_string(),
        });
    }
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

fn is_emoji(c: char) -> bool {
    let code = c as u32;
    // 囲み文字・記号類（🈑🈔など）
    (0x1F100..=0x1F1FF).contains(&code)
        || (0x1F200..=0x1F2FF).contains(&code)
        || (0x1F300..=0x1F9FF).contains(&code)
        || (0x2600..=0x26FF).contains(&code)
        || (0x2700..=0x27BF).contains(&code)
}

fn strip_emoji(s: &str) -> String {
    s.chars().filter(|c| !is_emoji(*c)).collect()
}

fn string_width(s: &str) -> usize {
    UnicodeWidthStr::width(strip_emoji(s).as_str())
}

fn truncate_string(s: &str, max_width: usize) -> String {
    let cleaned = strip_emoji(s);
    let mut result = String::new();
    let mut width = 0;
    for c in cleaned.chars() {
        let char_width = UnicodeWidthChar::width(c).unwrap_or(0);
        if width + char_width > max_width {
            break;
        }
        result.push(c);
        width += char_width;
    }
    result
}

fn pad_string(s: &str, target_width: usize) -> String {
    let current_width = string_width(s);
    if current_width >= target_width {
        truncate_string(s, target_width)
    } else {
        // 幅は絵文字を除いて数えるので、出力側も揃えて両分岐の契約を一致させる
        // （現在の呼び出し元は truncate_string 済みの文字列しか渡さないため防御的）
        let padding = target_width - current_width;
        format!("{}{}", strip_emoji(s), " ".repeat(padding))
    }
}

fn center_string(s: &str, target_width: usize) -> String {
    let current_width = string_width(s);
    if current_width >= target_width {
        truncate_string(s, target_width)
    } else {
        // 幅は絵文字を除いて数えるので、出力側も揃えて両分岐の契約を一致させる
        let total_padding = target_width - current_width;
        let left_padding = total_padding / 2;
        let right_padding = total_padding - left_padding;
        format!(
            "{}{}{}",
            " ".repeat(left_padding),
            strip_emoji(s),
            " ".repeat(right_padding)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn datetime(y: i32, m: u32, d: u32, h: u32, min: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
    }

    fn program(channel: usize, name: &str, start: &str, end: &str) -> Program {
        Program {
            id: String::new(),
            channel,
            name: name.to_string(),
            description: String::new(),
            link: String::new(),
            start_time: start.to_string(),
            end_time: end.to_string(),
        }
    }

    fn program_map(programs: &[Program]) -> BTreeMap<String, Program> {
        programs
            .iter()
            .map(|p| {
                (
                    format!("{}_{}", p.start_time, p.channel),
                    program(p.channel, &p.name, &p.start_time, &p.end_time),
                )
            })
            .collect()
    }

    fn group(programs: &[Program]) -> HashMap<usize, Vec<&Program>> {
        let mut m: HashMap<usize, Vec<&Program>> = HashMap::new();
        for p in programs {
            m.entry(p.channel).or_default().push(p);
        }
        m
    }

    // ---------- is_emoji ----------

    #[test]
    fn is_emoji_returns_false_for_plain_text() {
        for c in ['a', 'Z', '0', ' ', 'あ', '漢', '。'] {
            assert!(!is_emoji(c), "{:?} should not be treated as emoji", c);
        }
    }

    #[test]
    fn is_emoji_returns_false_for_box_drawing_characters() {
        // 表の罫線に使う文字が除去されると描画が壊れる
        for c in ['│', '─', '┼', '┤'] {
            assert!(!is_emoji(c), "{:?} should not be treated as emoji", c);
        }
    }

    #[test]
    fn is_emoji_covers_range_boundaries() {
        for code in [
            0x1F100, 0x1F1FF, 0x1F200, 0x1F2FF, 0x1F300, 0x1F9FF, 0x2600, 0x26FF, 0x2700, 0x27BF,
        ] {
            let c = char::from_u32(code).unwrap();
            assert!(is_emoji(c), "U+{:04X} should be inside a range", code);
        }
    }

    #[test]
    fn is_emoji_excludes_code_points_outside_ranges() {
        for code in [0x1F0FF, 0x1FA00, 0x25FF, 0x27C0, 0x2B50] {
            let c = char::from_u32(code).unwrap();
            assert!(!is_emoji(c), "U+{:04X} should be outside every range", code);
        }
    }

    #[test]
    fn is_emoji_matches_symbols_found_in_the_schedule() {
        assert!(is_emoji('🈑')); // U+1F211 「字」
        assert!(is_emoji('📺')); // U+1F4FA
        assert!(is_emoji('☀')); // U+2600
                                // 絵文字ではないが 0x2700..=0x27BF に入るため除去対象になる（現状の挙動）
        assert!(is_emoji('✓')); // U+2713
    }

    // ---------- strip_emoji ----------

    #[test]
    fn strip_emoji_removes_only_emoji_characters() {
        assert_eq!(strip_emoji("📺ニュース"), "ニュース");
        assert_eq!(strip_emoji("ニュース"), "ニュース");
        assert_eq!(strip_emoji("📺"), "");
        assert_eq!(strip_emoji(""), "");
    }

    // ---------- string_width ----------

    #[test]
    fn string_width_of_empty_string_is_zero() {
        assert_eq!(string_width(""), 0);
    }

    #[test]
    fn string_width_counts_ascii_as_one_each() {
        assert_eq!(string_width("abc"), 3);
    }

    #[test]
    fn string_width_counts_full_width_as_two_each() {
        assert_eq!(string_width("東京"), 4);
    }

    #[test]
    fn string_width_ignores_emoji() {
        assert_eq!(string_width("📺ドラマ"), 6);
        assert_eq!(string_width("📺"), 0);
    }

    // ---------- truncate_string ----------

    #[test]
    fn truncate_string_with_zero_width_returns_empty() {
        assert_eq!(truncate_string("ニュース", 0), "");
    }

    #[test]
    fn truncate_string_keeps_string_that_fits() {
        assert_eq!(truncate_string("news", 10), "news");
        assert_eq!(truncate_string("東京", 4), "東京");
    }

    #[test]
    fn truncate_string_cuts_ascii_at_max_width() {
        assert_eq!(truncate_string("abcdef", 3), "abc");
    }

    #[test]
    fn truncate_string_drops_full_width_char_straddling_the_limit() {
        // 全角は2幅なので境界をまたぐと丸ごと落ち、結果は max_width - 1 幅になる
        assert_eq!(truncate_string("東京", 3), "東");
        assert_eq!(string_width(&truncate_string("東京", 3)), 2);
    }

    #[test]
    fn truncate_string_removes_emoji_before_truncating() {
        assert_eq!(truncate_string("📺ニュース", 4), "ニュ");
    }

    #[test]
    fn truncate_string_of_empty_string_is_empty() {
        assert_eq!(truncate_string("", 5), "");
    }

    // ---------- pad_string ----------

    #[test]
    fn pad_string_right_pads_to_target_width() {
        assert_eq!(pad_string("ab", 5), "ab   ");
        assert_eq!(string_width(&pad_string("ab", 5)), 5);
    }

    #[test]
    fn pad_string_returns_string_of_exact_width_unchanged() {
        assert_eq!(pad_string("abcde", 5), "abcde");
    }

    #[test]
    fn pad_string_truncates_string_wider_than_target() {
        assert_eq!(pad_string("abcdefg", 3), "abc");
    }

    #[test]
    fn pad_string_pads_full_width_text_by_display_width() {
        assert_eq!(pad_string("東京", 6), "東京  ");
        assert_eq!(string_width(&pad_string("東京", 6)), 6);
    }

    #[test]
    fn pad_string_strips_emoji_so_the_cell_keeps_its_width() {
        assert_eq!(pad_string("📺A", 5), "A    ");
        assert_eq!(string_width(&pad_string("📺A", 5)), 5);
        assert_eq!(pad_string("📺A", 5).chars().count(), 5);
    }

    // ---------- center_string ----------

    #[test]
    fn center_string_splits_even_padding_evenly() {
        assert_eq!(center_string("ab", 6), "  ab  ");
    }

    #[test]
    fn center_string_puts_the_extra_space_on_the_right() {
        assert_eq!(center_string("abc", 6), " abc  ");
    }

    #[test]
    fn center_string_returns_string_of_exact_width_unchanged() {
        assert_eq!(center_string("abcd", 4), "abcd");
    }

    #[test]
    fn center_string_truncates_string_wider_than_target() {
        assert_eq!(center_string("東京", 3), "東");
    }

    #[test]
    fn center_string_strips_emoji_so_the_header_keeps_its_width() {
        assert_eq!(center_string("📺A", 5), "  A  ");
        assert_eq!(center_string("📺A", 5).chars().count(), 5);
    }

    // ---------- get_program_name / link / description ----------

    #[test]
    fn get_program_name_reads_the_program_title_element() {
        let html = scraper::Html::parse_fragment(
            r#"<div><a href="/si/1"><p class="program_title">ニュース</p></a></div>"#,
        );
        assert_eq!(get_program_name(&html), "ニュース");
    }

    #[test]
    fn get_program_name_returns_empty_when_absent() {
        let html = scraper::Html::parse_fragment("<div><p>no title here</p></div>");
        assert_eq!(get_program_name(&html), "");
    }

    #[test]
    fn get_program_name_takes_only_the_first_text_node() {
        // ネストしたマークアップでテキストが分割されると先頭ノードしか取れない（現状の挙動）
        let html = scraper::Html::parse_fragment(
            r#"<div><p class="program_title">ニュース<span>速報</span></p></div>"#,
        );
        assert_eq!(get_program_name(&html), "ニュース");
    }

    #[test]
    fn get_program_link_reads_the_href_attribute() {
        let html = scraper::Html::parse_fragment(r#"<div><a href="/si/12345">ニュース</a></div>"#);
        assert_eq!(get_program_link(&html), "/si/12345");
    }

    #[test]
    fn get_program_link_returns_empty_when_absent() {
        let html = scraper::Html::parse_fragment("<div><p>no anchor</p></div>");
        assert_eq!(get_program_link(&html), "");
    }

    #[test]
    fn get_program_description_reads_the_program_detail_element() {
        let html = scraper::Html::parse_fragment(
            r#"<div><p class="program_detail">今日のニュースをお伝えします</p></div>"#,
        );
        assert_eq!(
            get_program_description(&html),
            "今日のニュースをお伝えします"
        );
    }

    #[test]
    fn get_program_description_returns_empty_when_absent() {
        let html = scraper::Html::parse_fragment("<div><p>no detail</p></div>");
        assert_eq!(get_program_description(&html), "");
    }

    // ---------- get_channels ----------

    #[test]
    fn get_channels_collects_every_channel_name_in_order() {
        let html = scraper::Html::parse_document(
            r#"<div><div id="ch_area"><ul>
                 <li><p>NHK総合</p></li>
                 <li><p>日テレ</p></li>
                 <li><p>テレビ朝日</p></li>
               </ul></div></div>"#,
        );
        assert_eq!(get_channels(&html), vec!["NHK総合", "日テレ", "テレビ朝日"]);
    }

    #[test]
    fn get_channels_yields_empty_string_for_empty_element() {
        let html = scraper::Html::parse_document(
            r#"<div><div id="ch_area"><ul>
                 <li><p></p></li>
                 <li><p>日テレ</p></li>
               </ul></div></div>"#,
        );
        assert_eq!(get_channels(&html), vec!["", "日テレ"]);
    }

    #[test]
    fn get_channels_returns_empty_vec_when_ch_area_is_missing() {
        let html = scraper::Html::parse_document("<div><ul><li><p>NHK総合</p></li></ul></div>");
        assert_eq!(get_channels(&html), Vec::<String>::new());
    }

    // ---------- parse_programs ----------

    fn program_selector() -> scraper::Selector {
        scraper::Selector::parse("div #program_area ul li").unwrap()
    }

    fn epg_html(rows: &str) -> scraper::Html {
        scraper::Html::parse_document(&format!(
            r#"<div><div id="program_area">{}</div></div>"#,
            rows
        ))
    }

    #[test]
    fn parse_programs_reads_every_field_of_a_program() {
        let html = epg_html(
            r#"<ul id="program_line_1">
                 <li se-id="1234567ABCDE" s="202601101200" e="202601101300">
                   <div>
                     <a href="/si/12345"><p class="program_title">ニュース</p></a>
                     <p class="program_detail">今日のニュース</p>
                   </div>
                 </li>
               </ul>"#,
        );
        let mut programs = BTreeMap::new();
        parse_programs(&html, &program_selector(), &mut programs);

        assert_eq!(programs.len(), 1);
        assert_eq!(
            programs["202601101200_1"],
            Program {
                // se-id は先頭7文字を落とした残り
                id: "ABCDE".to_string(),
                channel: 1,
                name: "ニュース".to_string(),
                description: "今日のニュース".to_string(),
                link: "/si/12345".to_string(),
                start_time: "202601101200".to_string(),
                end_time: "202601101300".to_string(),
            }
        );
    }

    #[test]
    fn parse_programs_derives_the_channel_from_the_parent_id() {
        let html = epg_html(
            r#"<ul id="program_line_1">
                 <li s="202601101200" e="202601101300">
                   <div><p class="program_title">A</p></div>
                 </li>
               </ul>
               <ul id="program_line_5">
                 <li s="202601101200" e="202601101300">
                   <div><p class="program_title">B</p></div>
                 </li>
               </ul>"#,
        );
        let mut programs = BTreeMap::new();
        parse_programs(&html, &program_selector(), &mut programs);

        assert_eq!(programs.len(), 2);
        assert_eq!(programs["202601101200_1"].channel, 1);
        assert_eq!(programs["202601101200_1"].name, "A");
        assert_eq!(programs["202601101200_5"].channel, 5);
        assert_eq!(programs["202601101200_5"].name, "B");
    }

    #[test]
    fn parse_programs_skips_entries_without_a_name() {
        let html = epg_html(
            r#"<ul id="program_line_1">
                 <li s="202601101200" e="202601101300"><div></div></li>
                 <li s="202601101300" e="202601101400">
                   <div><p class="program_title">ドラマ</p></div>
                 </li>
               </ul>"#,
        );
        let mut programs = BTreeMap::new();
        parse_programs(&html, &program_selector(), &mut programs);

        assert_eq!(programs.len(), 1);
        assert!(programs.contains_key("202601101300_1"));
    }

    #[test]
    fn parse_programs_does_not_overwrite_an_existing_key() {
        // 2日分のページを続けて読むため、同じ開始時刻・同じチャンネルは先勝ちで残す
        let day1 = epg_html(
            r#"<ul id="program_line_1">
                 <li s="202601101200" e="202601101300">
                   <div><p class="program_title">1日目</p></div>
                 </li>
               </ul>"#,
        );
        let day2 = epg_html(
            r#"<ul id="program_line_1">
                 <li s="202601101200" e="202601101300">
                   <div><p class="program_title">2日目</p></div>
                 </li>
               </ul>"#,
        );
        let mut programs = BTreeMap::new();
        parse_programs(&day1, &program_selector(), &mut programs);
        parse_programs(&day2, &program_selector(), &mut programs);

        assert_eq!(programs.len(), 1);
        assert_eq!(programs["202601101200_1"].name, "1日目");
    }

    #[test]
    fn parse_programs_yields_empty_id_when_se_id_is_shorter_than_seven_chars() {
        let html = epg_html(
            r#"<ul id="program_line_1">
                 <li se-id="123" s="202601101200" e="202601101300">
                   <div><p class="program_title">ニュース</p></div>
                 </li>
               </ul>"#,
        );
        let mut programs = BTreeMap::new();
        parse_programs(&html, &program_selector(), &mut programs);

        assert_eq!(programs["202601101200_1"].id, "");
    }

    #[test]
    fn parse_programs_yields_empty_times_when_attributes_are_missing() {
        let html = epg_html(
            r#"<ul id="program_line_1">
                 <li><div><p class="program_title">ニュース</p></div></li>
               </ul>"#,
        );
        let mut programs = BTreeMap::new();
        parse_programs(&html, &program_selector(), &mut programs);

        let p = &programs["_1"];
        assert_eq!(p.start_time, "");
        assert_eq!(p.end_time, "");
        assert_eq!(p.id, "");
    }

    #[test]
    fn parse_programs_skips_rows_whose_parent_has_no_id() {
        let html = epg_html(
            r#"<ul>
                 <li s="202601101200" e="202601101300">
                   <div><p class="program_title">ニュース</p></div>
                 </li>
               </ul>"#,
        );
        let mut programs = BTreeMap::new();
        parse_programs(&html, &program_selector(), &mut programs);
        assert!(programs.is_empty());
    }

    #[test]
    fn parse_programs_skips_rows_whose_parent_id_is_not_numeric() {
        let html = epg_html(
            r#"<ul id="program_line_abc">
                 <li s="202601101200" e="202601101300">
                   <div><p class="program_title">ニュース</p></div>
                 </li>
               </ul>"#,
        );
        let mut programs = BTreeMap::new();
        parse_programs(&html, &program_selector(), &mut programs);
        assert!(programs.is_empty());
    }

    #[test]
    fn parse_programs_skips_rows_whose_parent_id_only_contains_the_prefix() {
        // replace ではなく strip_prefix なので、接頭辞が繰り返される id は弾く
        let html = epg_html(
            r#"<ul id="program_line_1program_line_">
                 <li s="202601101200" e="202601101300">
                   <div><p class="program_title">ニュース</p></div>
                 </li>
               </ul>
               <ul id="xprogram_line_2">
                 <li s="202601101200" e="202601101300">
                   <div><p class="program_title">ニュース</p></div>
                 </li>
               </ul>"#,
        );
        let mut programs = BTreeMap::new();
        parse_programs(&html, &program_selector(), &mut programs);
        assert!(programs.is_empty());
    }

    #[test]
    fn parse_programs_keeps_valid_rows_alongside_malformed_ones() {
        // 一部の行が壊れていても残りの番組表は表示できる
        let html = epg_html(
            r#"<ul id="sidebar">
                 <li s="202601101200" e="202601101300">
                   <div><p class="program_title">壊れた行</p></div>
                 </li>
               </ul>
               <ul id="program_line_2">
                 <li s="202601101200" e="202601101300">
                   <div><p class="program_title">正常な行</p></div>
                 </li>
               </ul>"#,
        );
        let mut programs = BTreeMap::new();
        parse_programs(&html, &program_selector(), &mut programs);

        assert_eq!(programs.len(), 1);
        assert_eq!(programs["202601101200_2"].name, "正常な行");
    }

    // ---------- resolve_area_id / area_names ----------

    #[test]
    fn resolve_area_id_maps_known_areas() {
        assert_eq!(resolve_area_id("東京"), Some("42"));
        assert_eq!(resolve_area_id("福岡"), Some("117"));
        assert_eq!(resolve_area_id("北九州"), Some("120"));
        assert_eq!(resolve_area_id("札幌"), Some("1"));
    }

    #[test]
    fn resolve_area_id_returns_none_for_unknown_area() {
        assert_eq!(resolve_area_id("存在しない県"), None);
        assert_eq!(resolve_area_id(""), None);
        assert_eq!(resolve_area_id("Tokyo"), None);
    }

    #[test]
    fn area_names_lists_areas_in_declaration_order_separated_by_spaces() {
        // HashMap のランダム順から宣言順に変えたのが今回唯一の意図的な挙動変更
        let expected: Vec<&str> = AREA_IDS.iter().map(|(name, _)| *name).collect();
        assert_eq!(area_names().split(' ').collect::<Vec<&str>>(), expected);
        assert!(area_names().starts_with("札幌 函館 旭川 帯広"));
        assert!(area_names().ends_with("沖縄 北九州"));
    }

    #[test]
    fn area_ids_covers_all_54_areas_with_unique_names_and_ids() {
        assert_eq!(AREA_IDS.len(), 54);

        let mut names: Vec<&str> = AREA_IDS.iter().map(|(name, _)| *name).collect();
        names.sort();
        let total = names.len();
        names.dedup();
        assert_eq!(names.len(), total, "エリア名が重複している");

        let mut ids: Vec<&str> = AREA_IDS.iter().map(|(_, id)| *id).collect();
        ids.sort();
        let total = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), total, "ggm_group_id が重複している");

        // 4件だけでなく全件が引けることを確認する
        for (name, id) in AREA_IDS {
            assert_eq!(resolve_area_id(name), Some(*id), "{} が引けない", name);
        }
    }

    // ---------- epg_url ----------

    #[test]
    fn epg_url_builds_the_bangumi_org_query() {
        assert_eq!(
            epg_url("20260110", "42"),
            "https://bangumi.org/epg/td?broad_cast_date=20260110&ggm_group_id=42"
        );
    }

    // ---------- resolve_dates ----------

    #[test]
    fn resolve_dates_before_5am_uses_the_previous_day() {
        // 番組表は5時起点なので 04:30 は前日の番組表に属する
        let d = resolve_dates(datetime(2026, 1, 10, 4, 30), 12);
        assert_eq!(
            d,
            Dates {
                date: "20260109".to_string(),
                next_date: "20260110".to_string(),
                needs_next_day: true,
            }
        );
    }

    #[test]
    fn resolve_dates_before_5am_within_the_same_broadcast_day() {
        // 03:00 + 1h = 04:00 なのでまだ前日の番組表に収まる
        let d = resolve_dates(datetime(2026, 1, 10, 3, 0), 1);
        assert_eq!(
            d,
            Dates {
                date: "20260109".to_string(),
                next_date: "20260110".to_string(),
                needs_next_day: false,
            }
        );
    }

    #[test]
    fn resolve_dates_before_5am_landing_exactly_on_5am_needs_the_next_day() {
        // 04:00 + 1h = 05:00 ちょうど。境界は >= なので当日分の取得が必要
        let d = resolve_dates(datetime(2026, 1, 10, 4, 0), 1);
        assert!(d.needs_next_day);
        assert_eq!(d.date, "20260109");
        assert_eq!(d.next_date, "20260110");
    }

    #[test]
    fn resolve_dates_landing_exactly_on_next_5am_needs_the_next_day() {
        // 17:00 + 12h = 翌日05:00 ちょうど。こちらも境界は >=
        let d = resolve_dates(datetime(2026, 1, 10, 17, 0), 12);
        assert!(d.needs_next_day);
        assert_eq!(d.next_date, "20260111");
    }

    #[test]
    fn resolve_dates_at_5am_belongs_to_the_same_day() {
        // 05:00 ちょうどは当日の番組表（判定は < 5）
        let d = resolve_dates(datetime(2026, 1, 10, 5, 0), 1);
        assert_eq!(d.date, "20260110");
    }

    #[test]
    fn resolve_dates_during_the_day_needs_no_extra_fetch() {
        let d = resolve_dates(datetime(2026, 1, 10, 10, 0), 12);
        assert_eq!(
            d,
            Dates {
                date: "20260110".to_string(),
                next_date: "20260111".to_string(),
                needs_next_day: false,
            }
        );
    }

    #[test]
    fn resolve_dates_crossing_next_5am_needs_the_next_day() {
        // 20:00 + 12h = 翌日08:00 なので翌日の番組表も必要
        let d = resolve_dates(datetime(2026, 1, 10, 20, 0), 12);
        assert_eq!(
            d,
            Dates {
                date: "20260110".to_string(),
                next_date: "20260111".to_string(),
                needs_next_day: true,
            }
        );
    }

    #[test]
    fn resolve_dates_past_midnight_but_before_5am_stays_on_the_same_day() {
        // 23:00 + 3h = 翌日02:00。日付は変わるが5時前なので同じ番組表のまま
        let d = resolve_dates(datetime(2026, 1, 10, 23, 0), 3);
        assert!(!d.needs_next_day);
        assert_eq!(d.date, "20260110");
    }

    #[test]
    fn resolve_dates_rolls_over_month_and_year_boundaries() {
        let d = resolve_dates(datetime(2026, 1, 1, 2, 0), 1);
        assert_eq!(d.date, "20251231");
        assert_eq!(d.next_date, "20260101");
    }

    // ---------- filter_programs ----------

    #[test]
    fn filter_programs_drops_programs_that_already_ended() {
        let programs = program_map(&[
            program(1, "終了済み", "202601101000", "202601101100"),
            program(1, "放送中", "202601101100", "202601101300"),
        ]);
        let grouped = filter_programs(&programs, "202601101200", "202601102000");

        let names: Vec<&str> = grouped[&1].iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["放送中"]);
    }

    #[test]
    fn filter_programs_keeps_a_program_ending_exactly_now() {
        // 判定は end_time < now_str なので、ちょうど今終わる番組はまだ表示する
        let programs = program_map(&[program(1, "ちょうど終了", "202601101100", "202601101200")]);
        let grouped = filter_programs(&programs, "202601101200", "202601102000");
        assert_eq!(grouped[&1].len(), 1);
    }

    #[test]
    fn filter_programs_keeps_a_program_starting_exactly_at_the_target() {
        // 判定は start_time > target なので、ちょうど境界に始まる番組は表示する
        let programs = program_map(&[program(1, "境界", "202601102000", "202601102100")]);
        let grouped = filter_programs(&programs, "202601101200", "202601102000");
        assert_eq!(grouped[&1].len(), 1);
    }

    #[test]
    fn filter_programs_drops_programs_starting_after_the_target() {
        let programs = program_map(&[
            program(1, "範囲内", "202601101300", "202601101400"),
            program(1, "範囲外", "202601102100", "202601102200"),
        ]);
        let grouped = filter_programs(&programs, "202601101200", "202601102000");

        let names: Vec<&str> = grouped[&1].iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["範囲内"]);
    }

    #[test]
    fn filter_programs_groups_by_channel_in_start_time_order() {
        let programs = program_map(&[
            program(2, "ch2 後", "202601101400", "202601101500"),
            program(1, "ch1 先", "202601101200", "202601101300"),
            program(2, "ch2 先", "202601101200", "202601101300"),
        ]);
        let grouped = filter_programs(&programs, "202601101200", "202601102000");

        assert_eq!(grouped.len(), 2);
        assert_eq!(
            grouped[&1]
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            vec!["ch1 先"]
        );
        assert_eq!(
            grouped[&2]
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            vec!["ch2 先", "ch2 後"]
        );
    }

    #[test]
    fn filter_programs_returns_empty_map_when_nothing_matches() {
        let programs = program_map(&[program(1, "終了済み", "202601101000", "202601101100")]);
        let grouped = filter_programs(&programs, "202601101200", "202601102000");
        assert!(grouped.is_empty());
    }

    #[test]
    fn filter_programs_drops_programs_parsed_without_time_attributes() {
        // s/e 属性が欠けた番組は start/end が空文字になり、必ず絞り込みで落ちる
        let html = epg_html(
            r#"<ul id="program_line_1">
                 <li><div><p class="program_title">属性なし</p></div></li>
               </ul>"#,
        );
        let mut programs = BTreeMap::new();
        parse_programs(&html, &program_selector(), &mut programs);
        assert_eq!(programs.len(), 1);

        let grouped = filter_programs(&programs, "202601101200", "202601102000");
        assert!(grouped.is_empty());
    }

    // ---------- visible_channels ----------

    #[test]
    fn visible_channels_sorts_channel_numbers_ascending() {
        let programs: Vec<Program> = [5usize, 1, 3]
            .iter()
            .map(|ch| program(*ch, "番組", "202601101200", "202601101300"))
            .collect();
        assert_eq!(visible_channels(&group(&programs)), vec![1, 3, 5]);
    }

    #[test]
    fn visible_channels_keeps_at_most_eight_channels() {
        let programs: Vec<Program> = (1usize..=12)
            .map(|ch| program(ch, "番組", "202601101200", "202601101300"))
            .collect();
        assert_eq!(
            visible_channels(&group(&programs)),
            vec![1, 2, 3, 4, 5, 6, 7, 8]
        );
    }

    #[test]
    fn visible_channels_returns_empty_for_empty_input() {
        let empty: HashMap<usize, Vec<&Program>> = HashMap::new();
        assert_eq!(visible_channels(&empty), Vec::<usize>::new());
    }

    // ---------- render_schedule ----------

    fn rendered() -> String {
        let channels = vec!["NHK総合".to_string(), "日テレ".to_string()];
        let programs = vec![program(1, "ニュース", "202601101200", "202601101300")];
        let grouped = group(&programs);
        render_schedule(&channels, &grouped, &[1, 2], 12, 0)
    }

    #[test]
    fn render_schedule_writes_a_header_and_separator_then_two_lines_per_hour() {
        let out = rendered();
        // hours = 0 なので 12時の1時間分（2行）+ ヘッダー + 区切り線
        assert_eq!(out.lines().count(), 4);
        assert!(out.ends_with('\n'));
    }

    #[test]
    fn render_schedule_matches_the_expected_table_layout() {
        // 定数から組み立てず、列幅の絶対値ごと出力全体を固定する
        assert_eq!(
            rendered(),
            concat!(
                "           \u{2502}      NHK総合       \u{2502}       日テレ       \u{2502}\n",
                "───────────┼────────────────────┼────────────────────┤\n",
                " 12:00     \u{2502}12:00 ニュース      \u{2502}                    \u{2502}\n",
                "           \u{2502}                    \u{2502}                    \u{2502}\n",
            )
        );
    }

    #[test]
    fn render_schedule_centers_channel_names_in_the_header() {
        let out = rendered();
        let header = out.lines().next().unwrap();
        assert!(header.starts_with(&" ".repeat(TIME_WIDTH)));
        assert!(header.contains("NHK総合"));
        assert!(header.contains("日テレ"));
        assert!(header.ends_with('│'));
    }

    #[test]
    fn render_schedule_draws_a_separator_matching_the_column_layout() {
        let out = rendered();
        let separator = out.lines().nth(1).unwrap();
        let expected = format!(
            "{}{}┤",
            "─".repeat(TIME_WIDTH),
            format!("┼{}", "─".repeat(COL_WIDTH)).repeat(2)
        );
        assert_eq!(separator, expected);
    }

    #[test]
    fn render_schedule_prefixes_the_program_cell_with_its_start_time() {
        let out = rendered();
        let row = out.lines().nth(2).unwrap();
        assert!(row.starts_with(" 12:00     "));
        assert!(row.contains("12:00 ニュース"));
    }

    #[test]
    fn render_schedule_leaves_channels_without_a_program_blank() {
        let out = rendered();
        let row = out.lines().nth(2).unwrap();
        // ch2 には番組がないので空白セルになる
        assert!(row.ends_with(&format!("│{}│", " ".repeat(COL_WIDTH))));
    }

    #[test]
    fn render_schedule_keeps_every_line_at_the_same_display_width() {
        let out = rendered();
        let expected = TIME_WIDTH + 2 * (1 + COL_WIDTH) + 1;
        for line in out.lines() {
            assert_eq!(string_width(line), expected, "line width differs: {}", line);
        }
    }

    #[test]
    fn render_schedule_keeps_the_width_even_with_emoji_in_names() {
        // string_width は絵文字を除いて数えるため、ここでは生の表示幅で検証する
        let channels = vec!["🈑NHK総合".to_string()];
        let programs = vec![program(
            1,
            "🈔📺ニュース速報",
            "202601101200",
            "202601101300",
        )];
        let out = render_schedule(&channels, &group(&programs), &[1], 12, 0);
        let expected = TIME_WIDTH + (1 + COL_WIDTH) + 1;

        for line in out.lines() {
            assert!(
                !line.chars().any(is_emoji),
                "emoji leaked into the table: {}",
                line
            );
            assert_eq!(UnicodeWidthStr::width(line), expected, "line: {}", line);
        }
    }

    #[test]
    fn render_schedule_renders_a_degenerate_table_when_no_channel_is_visible() {
        // 該当番組が1件もない時間帯でも表の枠は崩れない
        let out = render_schedule(&[], &HashMap::new(), &[], 12, 0);
        assert_eq!(
            out,
            "           \u{2502}\n───────────┤\n 12:00     \u{2502}\n           \u{2502}\n"
        );
    }

    #[test]
    fn render_schedule_shows_only_the_first_program_of_an_hour() {
        // 同じ時間帯に複数番組があっても1行1番組しか表示できない（現状の挙動）
        let channels = vec!["NHK総合".to_string()];
        let programs = vec![
            program(1, "前半", "202601101200", "202601101230"),
            program(1, "後半", "202601101230", "202601101300"),
        ];
        let out = render_schedule(&channels, &group(&programs), &[1], 12, 0);

        assert!(out.contains("12:00 前半"));
        assert!(!out.contains("後半"));
    }

    #[test]
    fn render_schedule_wraps_a_long_program_name_onto_the_second_line() {
        let channels = vec!["NHK総合".to_string()];
        let programs = vec![program(
            1,
            "あいうえおかきくけこさしすせそたちつてと",
            "202601101200",
            "202601101300",
        )];
        let out = render_schedule(&channels, &group(&programs), &[1], 12, 0);
        let lines: Vec<&str> = out.lines().collect();

        // 1行目のセルは COL_WIDTH - 7 = 13幅、全角なので6文字で打ち切られる
        assert_eq!(lines[2], " 12:00     │12:00 あいうえおか  │");
        // 続きは2行目に COL_WIDTH - 1 = 19幅、全角9文字ぶん入る
        assert_eq!(lines[3], "           │ きくけこさしすせそ │");
    }

    #[test]
    fn render_schedule_cuts_the_second_line_at_col_width_minus_one() {
        // 半角のみの番組名で 1行目13幅 / 2行目19幅の境界を固定する
        let channels = vec!["NHK総合".to_string()];
        let programs = vec![program(
            1,
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmn",
            "202601101200",
            "202601101300",
        )];
        let out = render_schedule(&channels, &group(&programs), &[1], 12, 0);
        let lines: Vec<&str> = out.lines().collect();

        assert_eq!(lines[2], " 12:00     │12:00 ABCDEFGHIJKLM │");
        assert_eq!(lines[3], "           │ NOPQRSTUVWXYZabcdef│");
    }

    #[test]
    fn render_schedule_falls_back_to_a_generic_label_for_unknown_channels() {
        let channels: Vec<String> = Vec::new();
        let programs = vec![program(3, "番組", "202601101200", "202601101300")];
        let out = render_schedule(&channels, &group(&programs), &[3], 12, 0);
        assert!(out.lines().next().unwrap().contains("Ch3"));
    }

    #[test]
    fn render_schedule_wraps_hours_past_midnight() {
        let channels = vec!["NHK総合".to_string()];
        let programs: Vec<Program> = Vec::new();
        let out = render_schedule(&channels, &group(&programs), &[1], 23, 2);
        let hours: Vec<&str> = out.lines().skip(2).step_by(2).map(|l| &l[..7]).collect();
        assert_eq!(hours, vec![" 23:00 ", " 00:00 ", " 01:00 "]);
    }
}
