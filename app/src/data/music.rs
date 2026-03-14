pub struct Track {
    pub title: &'static str,
    pub artist: &'static str,
    pub src: &'static str,
    pub artwork: &'static str,
    pub color: &'static str,
}

pub const TRACKS: &[Track] = &[
    Track { title: "Free to Change", artist: "Yuki Hamada", src: "/audio/free-to-change.mp3", artwork: "/assets/album-free-to-change.jpg", color: "#3b82f6" },
    Track { title: "HELLO 2150", artist: "Yuki Hamada", src: "/audio/hello-2150.mp3", artwork: "/assets/album-hello-2150.jpg", color: "#8b5cf6" },
    Track { title: "Everybody say BJJ", artist: "Yuki Hamada", src: "/audio/everybody-say-bjj.mp3", artwork: "/assets/album-everybody-bjj.jpg", color: "#ef4444" },
    Track { title: "I Love You", artist: "Yuki Hamada", src: "/audio/i-love-you.mp3", artwork: "/assets/album-i-love-you.jpg", color: "#ec4899" },
    Track { title: "I Need Your Attention", artist: "Yuki Hamada", src: "/audio/i-need-your-attention.mp3", artwork: "/assets/album-attention.jpg", color: "#f59e0b" },
    Track {
        title: "それ恋じゃなく柔術でした",
        artist: "Yuki Hamada",
        src: "/audio/sore-koi-janaku-jujutsu.mp3",
        artwork: "/assets/album-koi-jujutsu.jpg",
        color: "#10b981",
    },
    Track {
        title: "塩とピクセル",
        artist: "Yuki Hamada",
        src: "/audio/shio-to-pixel.mp3",
        artwork: "/assets/album-shio-pixel.jpg",
        color: "#06b6d4",
    },
    Track {
        title: "結び直す朝",
        artist: "Yuki Hamada",
        src: "/audio/musubinaosu-asa.mp3",
        artwork: "/assets/album-musubinaosu.jpg",
        color: "#f97316",
    },
];
