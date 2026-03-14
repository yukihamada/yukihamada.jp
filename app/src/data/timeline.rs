pub struct TimelineItem {
    pub key: &'static str,
    pub year: &'static str,
    pub link: Option<&'static str>,
    pub highlight: bool,
    pub logo_url: Option<&'static str>,
}

pub const TIMELINE: &[TimelineItem] = &[
    TimelineItem { key: "enabler", year: "2024~", link: Some("https://enablerhq.com"), highlight: true, logo_url: Some("/assets/logos/enabler.png") },
    TimelineItem { key: "reiwa", year: "2024~", link: Some("https://newt.net"), highlight: false, logo_url: Some("/assets/logos/reiwa.png") },
    TimelineItem { key: "giftmall", year: "~2024", link: Some("https://giftmall.co.jp"), highlight: false, logo_url: Some("/assets/logos/giftmall.png") },
    TimelineItem { key: "notahotel", year: "2018~2024", link: Some("https://notahotel.com"), highlight: false, logo_url: Some("/assets/logos/notahotel.png") },
    TimelineItem { key: "caster", year: "~2023", link: None, highlight: false, logo_url: Some("/assets/logos/caster.svg") },
    TimelineItem { key: "mercari", year: "2014~2021", link: None, highlight: false, logo_url: Some("/assets/logos/mercari.png") },
    TimelineItem { key: "cybridge", year: "2003~2013", link: None, highlight: false, logo_url: None },
];
