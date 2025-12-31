# tvshow

![crates](https://img.shields.io/crates/v/tvshow.svg) ![Licence](https://img.shields.io/github/license/Doarakko/tvshow)

Display Japanese TV schedules in the terminal.

## Install

### Cargo

```bash
cargo install tvshow
```

### Homebrew

```bash
brew install Doarakko/tap/tvshow
```

## Usage

```bash
tvshow -h
```

```
tvshow 0.1.2

USAGE:
    tvshow [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -a, --area <area>     [default: 東京]
```

## Example

```bash
tvshow -a 札幌

09/28 19時~
--------------------------------------------
19:56~22:54 【5 札幌テレビ1】	ものまねグランプリ 新世代ものまね歌姫＆最強の新ネタNo.1決定戦 [字] [15643]

09/28 21時~
--------------------------------------------
21:00~22:48 【8 北海道文化放送1】	お笑い王者が激推し!最強ピンネタ15連発[字]【陣内バカリ友近ゆりやん爆笑ピン芸人】 [49863]
21:54~23:10 【6 HTB1】	報道ステーション[字] [5989]

09/28 22時~
--------------------------------------------
22:00~22:57 【1 HBC北海道..】	THE百王【ひゃくおう】スシロー!王将!パティシエ!超人100秒早ワザSHOW[字] [4358]
22:00~22:45 【2 NHKEテレ..】	先人たちの底力 知恵泉 選「生活を支える新サービスを！小倉昌男」[解][字] [3183]
22:00~22:30 【3 NHK総合1..】	クローズアップ現代＋「史上最多のヒグマ被害はなぜ？追跡・都市部出没の真相とは」[字] [3072]
22:00~22:58 【7 TVh1】	WBS 中国不動産「裏取引」の現場!▼空港に有名作品続々…そのワケは?[字] [39714]
22:30~23:15 【3 NHK総合1..】	プロフェッショナル「一瞬に、心を込めて〜空港グランドスタッフ・中山弓子〜」[解][字] [3073]
22:45~22:55 【2 NHKEテレ..】	ココロほっこり おにぎり ミニ「山形 みそ焼きおにぎり」[字] [3186]
22:48~22:54 【8 北海道文化放送1】	8チャン!みちゅバチ [49864]
22:54~23:00 【5 札幌テレビ1】	STV天気予報 [15619]
22:54~23:00 【8 北海道文化放送1】	みんなのてんき [49865]
22:55~23:20 【2 NHKEテレ..】	テレビで中国語 イモトと学ぼう！中国語「第二回中間テスト！」 [3187]
22:57~23:00 【1 HBC北海道..】	HBC天気予報 [4246]
22:58~23:06 【7 TVh1】	私の食卓〜シゴト・メシ・ヒト〜[終] [41472]
```

## Credit

This TV schedule is got from [テレビ番組表 G ガイド](https://bangumi.org).

## Contributing

Before submitting a PR, run the CI checks locally:

```bash
# Build
cargo build --release

# Format
cargo fmt

# Lint
cargo clippy -- -D warnings
```

## Release

1. Update version in `Cargo.toml`
2. Push git tag
    ```bash
    git tag v0.1.3
    git push origin v0.1.3
    ```
3. Update Homebrew formula
    ```bash
    ./scripts/update-homebrew.sh 0.1.3
    ```

## Licence

MIT
