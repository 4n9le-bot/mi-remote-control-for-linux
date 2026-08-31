use super::LogicalKey;

pub const REGISTRY_LINUX_TAG: &str = "v6.16";
pub const REGISTRY_SOURCE_SHA256: &str =
    "7593bf38463ed7404fea256638c30ee74001dce986104b445b22b49873ebeadb";
pub const REGISTRY_LICENSE: &str = "GPL-2.0-only WITH Linux-syscall-note";
pub const REGISTRY_CATALOG_VERSION: u32 = 1;

pub static LOGICAL_KEYS: &[LogicalKey] = &[
    LogicalKey {
        symbol: "KEY_ESC",
        code: 1,
        label: "Esc",
    },
    LogicalKey {
        symbol: "KEY_1",
        code: 2,
        label: "1",
    },
    LogicalKey {
        symbol: "KEY_2",
        code: 3,
        label: "2",
    },
    LogicalKey {
        symbol: "KEY_3",
        code: 4,
        label: "3",
    },
    LogicalKey {
        symbol: "KEY_4",
        code: 5,
        label: "4",
    },
    LogicalKey {
        symbol: "KEY_5",
        code: 6,
        label: "5",
    },
    LogicalKey {
        symbol: "KEY_6",
        code: 7,
        label: "6",
    },
    LogicalKey {
        symbol: "KEY_7",
        code: 8,
        label: "7",
    },
    LogicalKey {
        symbol: "KEY_8",
        code: 9,
        label: "8",
    },
    LogicalKey {
        symbol: "KEY_9",
        code: 10,
        label: "9",
    },
    LogicalKey {
        symbol: "KEY_0",
        code: 11,
        label: "0",
    },
    LogicalKey {
        symbol: "KEY_MINUS",
        code: 12,
        label: "Minus",
    },
    LogicalKey {
        symbol: "KEY_EQUAL",
        code: 13,
        label: "Equal",
    },
    LogicalKey {
        symbol: "KEY_BACKSPACE",
        code: 14,
        label: "Backspace",
    },
    LogicalKey {
        symbol: "KEY_TAB",
        code: 15,
        label: "Tab",
    },
    LogicalKey {
        symbol: "KEY_Q",
        code: 16,
        label: "Q",
    },
    LogicalKey {
        symbol: "KEY_W",
        code: 17,
        label: "W",
    },
    LogicalKey {
        symbol: "KEY_E",
        code: 18,
        label: "E",
    },
    LogicalKey {
        symbol: "KEY_R",
        code: 19,
        label: "R",
    },
    LogicalKey {
        symbol: "KEY_T",
        code: 20,
        label: "T",
    },
    LogicalKey {
        symbol: "KEY_Y",
        code: 21,
        label: "Y",
    },
    LogicalKey {
        symbol: "KEY_U",
        code: 22,
        label: "U",
    },
    LogicalKey {
        symbol: "KEY_I",
        code: 23,
        label: "I",
    },
    LogicalKey {
        symbol: "KEY_O",
        code: 24,
        label: "O",
    },
    LogicalKey {
        symbol: "KEY_P",
        code: 25,
        label: "P",
    },
    LogicalKey {
        symbol: "KEY_LEFTBRACE",
        code: 26,
        label: "Leftbrace",
    },
    LogicalKey {
        symbol: "KEY_RIGHTBRACE",
        code: 27,
        label: "Rightbrace",
    },
    LogicalKey {
        symbol: "KEY_ENTER",
        code: 28,
        label: "Enter",
    },
    LogicalKey {
        symbol: "KEY_LEFTCTRL",
        code: 29,
        label: "Leftctrl",
    },
    LogicalKey {
        symbol: "KEY_A",
        code: 30,
        label: "A",
    },
    LogicalKey {
        symbol: "KEY_S",
        code: 31,
        label: "S",
    },
    LogicalKey {
        symbol: "KEY_D",
        code: 32,
        label: "D",
    },
    LogicalKey {
        symbol: "KEY_F",
        code: 33,
        label: "F",
    },
    LogicalKey {
        symbol: "KEY_G",
        code: 34,
        label: "G",
    },
    LogicalKey {
        symbol: "KEY_H",
        code: 35,
        label: "H",
    },
    LogicalKey {
        symbol: "KEY_J",
        code: 36,
        label: "J",
    },
    LogicalKey {
        symbol: "KEY_K",
        code: 37,
        label: "K",
    },
    LogicalKey {
        symbol: "KEY_L",
        code: 38,
        label: "L",
    },
    LogicalKey {
        symbol: "KEY_SEMICOLON",
        code: 39,
        label: "Semicolon",
    },
    LogicalKey {
        symbol: "KEY_APOSTROPHE",
        code: 40,
        label: "Apostrophe",
    },
    LogicalKey {
        symbol: "KEY_GRAVE",
        code: 41,
        label: "Grave",
    },
    LogicalKey {
        symbol: "KEY_LEFTSHIFT",
        code: 42,
        label: "Leftshift",
    },
    LogicalKey {
        symbol: "KEY_BACKSLASH",
        code: 43,
        label: "Backslash",
    },
    LogicalKey {
        symbol: "KEY_Z",
        code: 44,
        label: "Z",
    },
    LogicalKey {
        symbol: "KEY_X",
        code: 45,
        label: "X",
    },
    LogicalKey {
        symbol: "KEY_C",
        code: 46,
        label: "C",
    },
    LogicalKey {
        symbol: "KEY_V",
        code: 47,
        label: "V",
    },
    LogicalKey {
        symbol: "KEY_B",
        code: 48,
        label: "B",
    },
    LogicalKey {
        symbol: "KEY_N",
        code: 49,
        label: "N",
    },
    LogicalKey {
        symbol: "KEY_M",
        code: 50,
        label: "M",
    },
    LogicalKey {
        symbol: "KEY_COMMA",
        code: 51,
        label: "Comma",
    },
    LogicalKey {
        symbol: "KEY_DOT",
        code: 52,
        label: "Dot",
    },
    LogicalKey {
        symbol: "KEY_SLASH",
        code: 53,
        label: "Slash",
    },
    LogicalKey {
        symbol: "KEY_RIGHTSHIFT",
        code: 54,
        label: "Rightshift",
    },
    LogicalKey {
        symbol: "KEY_KPASTERISK",
        code: 55,
        label: "Kpasterisk",
    },
    LogicalKey {
        symbol: "KEY_LEFTALT",
        code: 56,
        label: "Leftalt",
    },
    LogicalKey {
        symbol: "KEY_SPACE",
        code: 57,
        label: "Space",
    },
    LogicalKey {
        symbol: "KEY_CAPSLOCK",
        code: 58,
        label: "Capslock",
    },
    LogicalKey {
        symbol: "KEY_F1",
        code: 59,
        label: "F1",
    },
    LogicalKey {
        symbol: "KEY_F2",
        code: 60,
        label: "F2",
    },
    LogicalKey {
        symbol: "KEY_F3",
        code: 61,
        label: "F3",
    },
    LogicalKey {
        symbol: "KEY_F4",
        code: 62,
        label: "F4",
    },
    LogicalKey {
        symbol: "KEY_F5",
        code: 63,
        label: "F5",
    },
    LogicalKey {
        symbol: "KEY_F6",
        code: 64,
        label: "F6",
    },
    LogicalKey {
        symbol: "KEY_F7",
        code: 65,
        label: "F7",
    },
    LogicalKey {
        symbol: "KEY_F8",
        code: 66,
        label: "F8",
    },
    LogicalKey {
        symbol: "KEY_F9",
        code: 67,
        label: "F9",
    },
    LogicalKey {
        symbol: "KEY_F10",
        code: 68,
        label: "F10",
    },
    LogicalKey {
        symbol: "KEY_NUMLOCK",
        code: 69,
        label: "Numlock",
    },
    LogicalKey {
        symbol: "KEY_SCROLLLOCK",
        code: 70,
        label: "Scrolllock",
    },
    LogicalKey {
        symbol: "KEY_KP7",
        code: 71,
        label: "Kp7",
    },
    LogicalKey {
        symbol: "KEY_KP8",
        code: 72,
        label: "Kp8",
    },
    LogicalKey {
        symbol: "KEY_KP9",
        code: 73,
        label: "Kp9",
    },
    LogicalKey {
        symbol: "KEY_KPMINUS",
        code: 74,
        label: "Kpminus",
    },
    LogicalKey {
        symbol: "KEY_KP4",
        code: 75,
        label: "Kp4",
    },
    LogicalKey {
        symbol: "KEY_KP5",
        code: 76,
        label: "Kp5",
    },
    LogicalKey {
        symbol: "KEY_KP6",
        code: 77,
        label: "Kp6",
    },
    LogicalKey {
        symbol: "KEY_KPPLUS",
        code: 78,
        label: "Kpplus",
    },
    LogicalKey {
        symbol: "KEY_KP1",
        code: 79,
        label: "Kp1",
    },
    LogicalKey {
        symbol: "KEY_KP2",
        code: 80,
        label: "Kp2",
    },
    LogicalKey {
        symbol: "KEY_KP3",
        code: 81,
        label: "Kp3",
    },
    LogicalKey {
        symbol: "KEY_KP0",
        code: 82,
        label: "Kp0",
    },
    LogicalKey {
        symbol: "KEY_KPDOT",
        code: 83,
        label: "Kpdot",
    },
    LogicalKey {
        symbol: "KEY_ZENKAKUHANKAKU",
        code: 85,
        label: "Zenkakuhankaku",
    },
    LogicalKey {
        symbol: "KEY_102ND",
        code: 86,
        label: "102Nd",
    },
    LogicalKey {
        symbol: "KEY_F11",
        code: 87,
        label: "F11",
    },
    LogicalKey {
        symbol: "KEY_F12",
        code: 88,
        label: "F12",
    },
    LogicalKey {
        symbol: "KEY_RO",
        code: 89,
        label: "Ro",
    },
    LogicalKey {
        symbol: "KEY_KATAKANA",
        code: 90,
        label: "Katakana",
    },
    LogicalKey {
        symbol: "KEY_HIRAGANA",
        code: 91,
        label: "Hiragana",
    },
    LogicalKey {
        symbol: "KEY_HENKAN",
        code: 92,
        label: "Henkan",
    },
    LogicalKey {
        symbol: "KEY_KATAKANAHIRAGANA",
        code: 93,
        label: "Katakanahiragana",
    },
    LogicalKey {
        symbol: "KEY_MUHENKAN",
        code: 94,
        label: "Muhenkan",
    },
    LogicalKey {
        symbol: "KEY_KPJPCOMMA",
        code: 95,
        label: "Kpjpcomma",
    },
    LogicalKey {
        symbol: "KEY_KPENTER",
        code: 96,
        label: "Kpenter",
    },
    LogicalKey {
        symbol: "KEY_RIGHTCTRL",
        code: 97,
        label: "Rightctrl",
    },
    LogicalKey {
        symbol: "KEY_KPSLASH",
        code: 98,
        label: "Kpslash",
    },
    LogicalKey {
        symbol: "KEY_SYSRQ",
        code: 99,
        label: "Sysrq",
    },
    LogicalKey {
        symbol: "KEY_RIGHTALT",
        code: 100,
        label: "Rightalt",
    },
    LogicalKey {
        symbol: "KEY_LINEFEED",
        code: 101,
        label: "Linefeed",
    },
    LogicalKey {
        symbol: "KEY_HOME",
        code: 102,
        label: "Home",
    },
    LogicalKey {
        symbol: "KEY_UP",
        code: 103,
        label: "Up",
    },
    LogicalKey {
        symbol: "KEY_PAGEUP",
        code: 104,
        label: "Pageup",
    },
    LogicalKey {
        symbol: "KEY_LEFT",
        code: 105,
        label: "Left",
    },
    LogicalKey {
        symbol: "KEY_RIGHT",
        code: 106,
        label: "Right",
    },
    LogicalKey {
        symbol: "KEY_END",
        code: 107,
        label: "End",
    },
    LogicalKey {
        symbol: "KEY_DOWN",
        code: 108,
        label: "Down",
    },
    LogicalKey {
        symbol: "KEY_PAGEDOWN",
        code: 109,
        label: "Pagedown",
    },
    LogicalKey {
        symbol: "KEY_INSERT",
        code: 110,
        label: "Insert",
    },
    LogicalKey {
        symbol: "KEY_DELETE",
        code: 111,
        label: "Delete",
    },
    LogicalKey {
        symbol: "KEY_MACRO",
        code: 112,
        label: "Macro",
    },
    LogicalKey {
        symbol: "KEY_MUTE",
        code: 113,
        label: "Mute",
    },
    LogicalKey {
        symbol: "KEY_VOLUMEDOWN",
        code: 114,
        label: "Volumedown",
    },
    LogicalKey {
        symbol: "KEY_VOLUMEUP",
        code: 115,
        label: "Volumeup",
    },
    LogicalKey {
        symbol: "KEY_POWER",
        code: 116,
        label: "Power",
    },
    LogicalKey {
        symbol: "KEY_KPEQUAL",
        code: 117,
        label: "Kpequal",
    },
    LogicalKey {
        symbol: "KEY_KPPLUSMINUS",
        code: 118,
        label: "Kpplusminus",
    },
    LogicalKey {
        symbol: "KEY_PAUSE",
        code: 119,
        label: "Pause",
    },
    LogicalKey {
        symbol: "KEY_SCALE",
        code: 120,
        label: "Scale",
    },
    LogicalKey {
        symbol: "KEY_KPCOMMA",
        code: 121,
        label: "Kpcomma",
    },
    LogicalKey {
        symbol: "KEY_HANGEUL",
        code: 122,
        label: "Hangeul",
    },
    LogicalKey {
        symbol: "KEY_HANJA",
        code: 123,
        label: "Hanja",
    },
    LogicalKey {
        symbol: "KEY_YEN",
        code: 124,
        label: "Yen",
    },
    LogicalKey {
        symbol: "KEY_LEFTMETA",
        code: 125,
        label: "Leftmeta",
    },
    LogicalKey {
        symbol: "KEY_RIGHTMETA",
        code: 126,
        label: "Rightmeta",
    },
    LogicalKey {
        symbol: "KEY_COMPOSE",
        code: 127,
        label: "Compose",
    },
    LogicalKey {
        symbol: "KEY_STOP",
        code: 128,
        label: "Stop",
    },
    LogicalKey {
        symbol: "KEY_AGAIN",
        code: 129,
        label: "Again",
    },
    LogicalKey {
        symbol: "KEY_PROPS",
        code: 130,
        label: "Props",
    },
    LogicalKey {
        symbol: "KEY_UNDO",
        code: 131,
        label: "Undo",
    },
    LogicalKey {
        symbol: "KEY_FRONT",
        code: 132,
        label: "Front",
    },
    LogicalKey {
        symbol: "KEY_COPY",
        code: 133,
        label: "Copy",
    },
    LogicalKey {
        symbol: "KEY_OPEN",
        code: 134,
        label: "Open",
    },
    LogicalKey {
        symbol: "KEY_PASTE",
        code: 135,
        label: "Paste",
    },
    LogicalKey {
        symbol: "KEY_FIND",
        code: 136,
        label: "Find",
    },
    LogicalKey {
        symbol: "KEY_CUT",
        code: 137,
        label: "Cut",
    },
    LogicalKey {
        symbol: "KEY_HELP",
        code: 138,
        label: "Help",
    },
    LogicalKey {
        symbol: "KEY_MENU",
        code: 139,
        label: "Menu",
    },
    LogicalKey {
        symbol: "KEY_CALC",
        code: 140,
        label: "Calc",
    },
    LogicalKey {
        symbol: "KEY_SETUP",
        code: 141,
        label: "Setup",
    },
    LogicalKey {
        symbol: "KEY_SLEEP",
        code: 142,
        label: "Sleep",
    },
    LogicalKey {
        symbol: "KEY_WAKEUP",
        code: 143,
        label: "Wakeup",
    },
    LogicalKey {
        symbol: "KEY_FILE",
        code: 144,
        label: "File",
    },
    LogicalKey {
        symbol: "KEY_SENDFILE",
        code: 145,
        label: "Sendfile",
    },
    LogicalKey {
        symbol: "KEY_DELETEFILE",
        code: 146,
        label: "Deletefile",
    },
    LogicalKey {
        symbol: "KEY_XFER",
        code: 147,
        label: "Xfer",
    },
    LogicalKey {
        symbol: "KEY_PROG1",
        code: 148,
        label: "Prog1",
    },
    LogicalKey {
        symbol: "KEY_PROG2",
        code: 149,
        label: "Prog2",
    },
    LogicalKey {
        symbol: "KEY_WWW",
        code: 150,
        label: "Www",
    },
    LogicalKey {
        symbol: "KEY_MSDOS",
        code: 151,
        label: "Msdos",
    },
    LogicalKey {
        symbol: "KEY_COFFEE",
        code: 152,
        label: "Coffee",
    },
    LogicalKey {
        symbol: "KEY_ROTATE_DISPLAY",
        code: 153,
        label: "Rotate Display",
    },
    LogicalKey {
        symbol: "KEY_CYCLEWINDOWS",
        code: 154,
        label: "Cyclewindows",
    },
    LogicalKey {
        symbol: "KEY_MAIL",
        code: 155,
        label: "Mail",
    },
    LogicalKey {
        symbol: "KEY_BOOKMARKS",
        code: 156,
        label: "Bookmarks",
    },
    LogicalKey {
        symbol: "KEY_COMPUTER",
        code: 157,
        label: "Computer",
    },
    LogicalKey {
        symbol: "KEY_BACK",
        code: 158,
        label: "Back",
    },
    LogicalKey {
        symbol: "KEY_FORWARD",
        code: 159,
        label: "Forward",
    },
    LogicalKey {
        symbol: "KEY_CLOSECD",
        code: 160,
        label: "Closecd",
    },
    LogicalKey {
        symbol: "KEY_EJECTCD",
        code: 161,
        label: "Ejectcd",
    },
    LogicalKey {
        symbol: "KEY_EJECTCLOSECD",
        code: 162,
        label: "Ejectclosecd",
    },
    LogicalKey {
        symbol: "KEY_NEXTSONG",
        code: 163,
        label: "Nextsong",
    },
    LogicalKey {
        symbol: "KEY_PLAYPAUSE",
        code: 164,
        label: "Playpause",
    },
    LogicalKey {
        symbol: "KEY_PREVIOUSSONG",
        code: 165,
        label: "Previoussong",
    },
    LogicalKey {
        symbol: "KEY_STOPCD",
        code: 166,
        label: "Stopcd",
    },
    LogicalKey {
        symbol: "KEY_RECORD",
        code: 167,
        label: "Record",
    },
    LogicalKey {
        symbol: "KEY_REWIND",
        code: 168,
        label: "Rewind",
    },
    LogicalKey {
        symbol: "KEY_PHONE",
        code: 169,
        label: "Phone",
    },
    LogicalKey {
        symbol: "KEY_ISO",
        code: 170,
        label: "Iso",
    },
    LogicalKey {
        symbol: "KEY_CONFIG",
        code: 171,
        label: "Config",
    },
    LogicalKey {
        symbol: "KEY_HOMEPAGE",
        code: 172,
        label: "Homepage",
    },
    LogicalKey {
        symbol: "KEY_REFRESH",
        code: 173,
        label: "Refresh",
    },
    LogicalKey {
        symbol: "KEY_EXIT",
        code: 174,
        label: "Exit",
    },
    LogicalKey {
        symbol: "KEY_MOVE",
        code: 175,
        label: "Move",
    },
    LogicalKey {
        symbol: "KEY_EDIT",
        code: 176,
        label: "Edit",
    },
    LogicalKey {
        symbol: "KEY_SCROLLUP",
        code: 177,
        label: "Scrollup",
    },
    LogicalKey {
        symbol: "KEY_SCROLLDOWN",
        code: 178,
        label: "Scrolldown",
    },
    LogicalKey {
        symbol: "KEY_KPLEFTPAREN",
        code: 179,
        label: "Kpleftparen",
    },
    LogicalKey {
        symbol: "KEY_KPRIGHTPAREN",
        code: 180,
        label: "Kprightparen",
    },
    LogicalKey {
        symbol: "KEY_NEW",
        code: 181,
        label: "New",
    },
    LogicalKey {
        symbol: "KEY_REDO",
        code: 182,
        label: "Redo",
    },
    LogicalKey {
        symbol: "KEY_F13",
        code: 183,
        label: "F13",
    },
    LogicalKey {
        symbol: "KEY_F14",
        code: 184,
        label: "F14",
    },
    LogicalKey {
        symbol: "KEY_F15",
        code: 185,
        label: "F15",
    },
    LogicalKey {
        symbol: "KEY_F16",
        code: 186,
        label: "F16",
    },
    LogicalKey {
        symbol: "KEY_F17",
        code: 187,
        label: "F17",
    },
    LogicalKey {
        symbol: "KEY_F18",
        code: 188,
        label: "F18",
    },
    LogicalKey {
        symbol: "KEY_F19",
        code: 189,
        label: "F19",
    },
    LogicalKey {
        symbol: "KEY_F20",
        code: 190,
        label: "F20",
    },
    LogicalKey {
        symbol: "KEY_F21",
        code: 191,
        label: "F21",
    },
    LogicalKey {
        symbol: "KEY_F22",
        code: 192,
        label: "F22",
    },
    LogicalKey {
        symbol: "KEY_F23",
        code: 193,
        label: "F23",
    },
    LogicalKey {
        symbol: "KEY_F24",
        code: 194,
        label: "F24",
    },
    LogicalKey {
        symbol: "KEY_PLAYCD",
        code: 200,
        label: "Playcd",
    },
    LogicalKey {
        symbol: "KEY_PAUSECD",
        code: 201,
        label: "Pausecd",
    },
    LogicalKey {
        symbol: "KEY_PROG3",
        code: 202,
        label: "Prog3",
    },
    LogicalKey {
        symbol: "KEY_PROG4",
        code: 203,
        label: "Prog4",
    },
    LogicalKey {
        symbol: "KEY_ALL_APPLICATIONS",
        code: 204,
        label: "All Applications",
    },
    LogicalKey {
        symbol: "KEY_SUSPEND",
        code: 205,
        label: "Suspend",
    },
    LogicalKey {
        symbol: "KEY_CLOSE",
        code: 206,
        label: "Close",
    },
    LogicalKey {
        symbol: "KEY_PLAY",
        code: 207,
        label: "Play",
    },
    LogicalKey {
        symbol: "KEY_FASTFORWARD",
        code: 208,
        label: "Fastforward",
    },
    LogicalKey {
        symbol: "KEY_BASSBOOST",
        code: 209,
        label: "Bassboost",
    },
    LogicalKey {
        symbol: "KEY_PRINT",
        code: 210,
        label: "Print",
    },
    LogicalKey {
        symbol: "KEY_HP",
        code: 211,
        label: "Hp",
    },
    LogicalKey {
        symbol: "KEY_CAMERA",
        code: 212,
        label: "Camera",
    },
    LogicalKey {
        symbol: "KEY_SOUND",
        code: 213,
        label: "Sound",
    },
    LogicalKey {
        symbol: "KEY_QUESTION",
        code: 214,
        label: "Question",
    },
    LogicalKey {
        symbol: "KEY_EMAIL",
        code: 215,
        label: "Email",
    },
    LogicalKey {
        symbol: "KEY_CHAT",
        code: 216,
        label: "Chat",
    },
    LogicalKey {
        symbol: "KEY_SEARCH",
        code: 217,
        label: "Search",
    },
    LogicalKey {
        symbol: "KEY_CONNECT",
        code: 218,
        label: "Connect",
    },
    LogicalKey {
        symbol: "KEY_FINANCE",
        code: 219,
        label: "Finance",
    },
    LogicalKey {
        symbol: "KEY_SPORT",
        code: 220,
        label: "Sport",
    },
    LogicalKey {
        symbol: "KEY_SHOP",
        code: 221,
        label: "Shop",
    },
    LogicalKey {
        symbol: "KEY_ALTERASE",
        code: 222,
        label: "Alterase",
    },
    LogicalKey {
        symbol: "KEY_CANCEL",
        code: 223,
        label: "Cancel",
    },
    LogicalKey {
        symbol: "KEY_BRIGHTNESSDOWN",
        code: 224,
        label: "Brightnessdown",
    },
    LogicalKey {
        symbol: "KEY_BRIGHTNESSUP",
        code: 225,
        label: "Brightnessup",
    },
    LogicalKey {
        symbol: "KEY_MEDIA",
        code: 226,
        label: "Media",
    },
    LogicalKey {
        symbol: "KEY_SWITCHVIDEOMODE",
        code: 227,
        label: "Switchvideomode",
    },
    LogicalKey {
        symbol: "KEY_KBDILLUMTOGGLE",
        code: 228,
        label: "Kbdillumtoggle",
    },
    LogicalKey {
        symbol: "KEY_KBDILLUMDOWN",
        code: 229,
        label: "Kbdillumdown",
    },
    LogicalKey {
        symbol: "KEY_KBDILLUMUP",
        code: 230,
        label: "Kbdillumup",
    },
    LogicalKey {
        symbol: "KEY_SEND",
        code: 231,
        label: "Send",
    },
    LogicalKey {
        symbol: "KEY_REPLY",
        code: 232,
        label: "Reply",
    },
    LogicalKey {
        symbol: "KEY_FORWARDMAIL",
        code: 233,
        label: "Forwardmail",
    },
    LogicalKey {
        symbol: "KEY_SAVE",
        code: 234,
        label: "Save",
    },
    LogicalKey {
        symbol: "KEY_DOCUMENTS",
        code: 235,
        label: "Documents",
    },
    LogicalKey {
        symbol: "KEY_BATTERY",
        code: 236,
        label: "Battery",
    },
    LogicalKey {
        symbol: "KEY_BLUETOOTH",
        code: 237,
        label: "Bluetooth",
    },
    LogicalKey {
        symbol: "KEY_WLAN",
        code: 238,
        label: "Wlan",
    },
    LogicalKey {
        symbol: "KEY_UWB",
        code: 239,
        label: "Uwb",
    },
    LogicalKey {
        symbol: "KEY_VIDEO_NEXT",
        code: 241,
        label: "Video Next",
    },
    LogicalKey {
        symbol: "KEY_VIDEO_PREV",
        code: 242,
        label: "Video Prev",
    },
    LogicalKey {
        symbol: "KEY_BRIGHTNESS_CYCLE",
        code: 243,
        label: "Brightness Cycle",
    },
    LogicalKey {
        symbol: "KEY_BRIGHTNESS_AUTO",
        code: 244,
        label: "Brightness Auto",
    },
    LogicalKey {
        symbol: "KEY_DISPLAY_OFF",
        code: 245,
        label: "Display Off",
    },
    LogicalKey {
        symbol: "KEY_WWAN",
        code: 246,
        label: "Wwan",
    },
    LogicalKey {
        symbol: "KEY_RFKILL",
        code: 247,
        label: "Rfkill",
    },
    LogicalKey {
        symbol: "KEY_MICMUTE",
        code: 248,
        label: "Micmute",
    },
    LogicalKey {
        symbol: "KEY_OK",
        code: 352,
        label: "Ok",
    },
    LogicalKey {
        symbol: "KEY_SELECT",
        code: 353,
        label: "Select",
    },
    LogicalKey {
        symbol: "KEY_GOTO",
        code: 354,
        label: "Goto",
    },
    LogicalKey {
        symbol: "KEY_CLEAR",
        code: 355,
        label: "Clear",
    },
    LogicalKey {
        symbol: "KEY_POWER2",
        code: 356,
        label: "Power2",
    },
    LogicalKey {
        symbol: "KEY_OPTION",
        code: 357,
        label: "Option",
    },
    LogicalKey {
        symbol: "KEY_INFO",
        code: 358,
        label: "Info",
    },
    LogicalKey {
        symbol: "KEY_TIME",
        code: 359,
        label: "Time",
    },
    LogicalKey {
        symbol: "KEY_VENDOR",
        code: 360,
        label: "Vendor",
    },
    LogicalKey {
        symbol: "KEY_ARCHIVE",
        code: 361,
        label: "Archive",
    },
    LogicalKey {
        symbol: "KEY_PROGRAM",
        code: 362,
        label: "Program",
    },
    LogicalKey {
        symbol: "KEY_CHANNEL",
        code: 363,
        label: "Channel",
    },
    LogicalKey {
        symbol: "KEY_FAVORITES",
        code: 364,
        label: "Favorites",
    },
    LogicalKey {
        symbol: "KEY_EPG",
        code: 365,
        label: "Epg",
    },
    LogicalKey {
        symbol: "KEY_PVR",
        code: 366,
        label: "Pvr",
    },
    LogicalKey {
        symbol: "KEY_MHP",
        code: 367,
        label: "Mhp",
    },
    LogicalKey {
        symbol: "KEY_LANGUAGE",
        code: 368,
        label: "Language",
    },
    LogicalKey {
        symbol: "KEY_TITLE",
        code: 369,
        label: "Title",
    },
    LogicalKey {
        symbol: "KEY_SUBTITLE",
        code: 370,
        label: "Subtitle",
    },
    LogicalKey {
        symbol: "KEY_ANGLE",
        code: 371,
        label: "Angle",
    },
    LogicalKey {
        symbol: "KEY_FULL_SCREEN",
        code: 372,
        label: "Full Screen",
    },
    LogicalKey {
        symbol: "KEY_MODE",
        code: 373,
        label: "Mode",
    },
    LogicalKey {
        symbol: "KEY_KEYBOARD",
        code: 374,
        label: "Keyboard",
    },
    LogicalKey {
        symbol: "KEY_ASPECT_RATIO",
        code: 375,
        label: "Aspect Ratio",
    },
    LogicalKey {
        symbol: "KEY_PC",
        code: 376,
        label: "Pc",
    },
    LogicalKey {
        symbol: "KEY_TV",
        code: 377,
        label: "Tv",
    },
    LogicalKey {
        symbol: "KEY_TV2",
        code: 378,
        label: "Tv2",
    },
    LogicalKey {
        symbol: "KEY_VCR",
        code: 379,
        label: "Vcr",
    },
    LogicalKey {
        symbol: "KEY_VCR2",
        code: 380,
        label: "Vcr2",
    },
    LogicalKey {
        symbol: "KEY_SAT",
        code: 381,
        label: "Sat",
    },
    LogicalKey {
        symbol: "KEY_SAT2",
        code: 382,
        label: "Sat2",
    },
    LogicalKey {
        symbol: "KEY_CD",
        code: 383,
        label: "Cd",
    },
    LogicalKey {
        symbol: "KEY_TAPE",
        code: 384,
        label: "Tape",
    },
    LogicalKey {
        symbol: "KEY_RADIO",
        code: 385,
        label: "Radio",
    },
    LogicalKey {
        symbol: "KEY_TUNER",
        code: 386,
        label: "Tuner",
    },
    LogicalKey {
        symbol: "KEY_PLAYER",
        code: 387,
        label: "Player",
    },
    LogicalKey {
        symbol: "KEY_TEXT",
        code: 388,
        label: "Text",
    },
    LogicalKey {
        symbol: "KEY_DVD",
        code: 389,
        label: "Dvd",
    },
    LogicalKey {
        symbol: "KEY_AUX",
        code: 390,
        label: "Aux",
    },
    LogicalKey {
        symbol: "KEY_MP3",
        code: 391,
        label: "Mp3",
    },
    LogicalKey {
        symbol: "KEY_AUDIO",
        code: 392,
        label: "Audio",
    },
    LogicalKey {
        symbol: "KEY_VIDEO",
        code: 393,
        label: "Video",
    },
    LogicalKey {
        symbol: "KEY_DIRECTORY",
        code: 394,
        label: "Directory",
    },
    LogicalKey {
        symbol: "KEY_LIST",
        code: 395,
        label: "List",
    },
    LogicalKey {
        symbol: "KEY_MEMO",
        code: 396,
        label: "Memo",
    },
    LogicalKey {
        symbol: "KEY_CALENDAR",
        code: 397,
        label: "Calendar",
    },
    LogicalKey {
        symbol: "KEY_RED",
        code: 398,
        label: "Red",
    },
    LogicalKey {
        symbol: "KEY_GREEN",
        code: 399,
        label: "Green",
    },
    LogicalKey {
        symbol: "KEY_YELLOW",
        code: 400,
        label: "Yellow",
    },
    LogicalKey {
        symbol: "KEY_BLUE",
        code: 401,
        label: "Blue",
    },
    LogicalKey {
        symbol: "KEY_CHANNELUP",
        code: 402,
        label: "Channelup",
    },
    LogicalKey {
        symbol: "KEY_CHANNELDOWN",
        code: 403,
        label: "Channeldown",
    },
    LogicalKey {
        symbol: "KEY_FIRST",
        code: 404,
        label: "First",
    },
    LogicalKey {
        symbol: "KEY_LAST",
        code: 405,
        label: "Last",
    },
    LogicalKey {
        symbol: "KEY_AB",
        code: 406,
        label: "Ab",
    },
    LogicalKey {
        symbol: "KEY_NEXT",
        code: 407,
        label: "Next",
    },
    LogicalKey {
        symbol: "KEY_RESTART",
        code: 408,
        label: "Restart",
    },
    LogicalKey {
        symbol: "KEY_SLOW",
        code: 409,
        label: "Slow",
    },
    LogicalKey {
        symbol: "KEY_SHUFFLE",
        code: 410,
        label: "Shuffle",
    },
    LogicalKey {
        symbol: "KEY_BREAK",
        code: 411,
        label: "Break",
    },
    LogicalKey {
        symbol: "KEY_PREVIOUS",
        code: 412,
        label: "Previous",
    },
    LogicalKey {
        symbol: "KEY_DIGITS",
        code: 413,
        label: "Digits",
    },
    LogicalKey {
        symbol: "KEY_TEEN",
        code: 414,
        label: "Teen",
    },
    LogicalKey {
        symbol: "KEY_TWEN",
        code: 415,
        label: "Twen",
    },
    LogicalKey {
        symbol: "KEY_VIDEOPHONE",
        code: 416,
        label: "Videophone",
    },
    LogicalKey {
        symbol: "KEY_GAMES",
        code: 417,
        label: "Games",
    },
    LogicalKey {
        symbol: "KEY_ZOOMIN",
        code: 418,
        label: "Zoomin",
    },
    LogicalKey {
        symbol: "KEY_ZOOMOUT",
        code: 419,
        label: "Zoomout",
    },
    LogicalKey {
        symbol: "KEY_ZOOMRESET",
        code: 420,
        label: "Zoomreset",
    },
    LogicalKey {
        symbol: "KEY_WORDPROCESSOR",
        code: 421,
        label: "Wordprocessor",
    },
    LogicalKey {
        symbol: "KEY_EDITOR",
        code: 422,
        label: "Editor",
    },
    LogicalKey {
        symbol: "KEY_SPREADSHEET",
        code: 423,
        label: "Spreadsheet",
    },
    LogicalKey {
        symbol: "KEY_GRAPHICSEDITOR",
        code: 424,
        label: "Graphicseditor",
    },
    LogicalKey {
        symbol: "KEY_PRESENTATION",
        code: 425,
        label: "Presentation",
    },
    LogicalKey {
        symbol: "KEY_DATABASE",
        code: 426,
        label: "Database",
    },
    LogicalKey {
        symbol: "KEY_NEWS",
        code: 427,
        label: "News",
    },
    LogicalKey {
        symbol: "KEY_VOICEMAIL",
        code: 428,
        label: "Voicemail",
    },
    LogicalKey {
        symbol: "KEY_ADDRESSBOOK",
        code: 429,
        label: "Addressbook",
    },
    LogicalKey {
        symbol: "KEY_MESSENGER",
        code: 430,
        label: "Messenger",
    },
    LogicalKey {
        symbol: "KEY_DISPLAYTOGGLE",
        code: 431,
        label: "Displaytoggle",
    },
    LogicalKey {
        symbol: "KEY_SPELLCHECK",
        code: 432,
        label: "Spellcheck",
    },
    LogicalKey {
        symbol: "KEY_LOGOFF",
        code: 433,
        label: "Logoff",
    },
    LogicalKey {
        symbol: "KEY_DOLLAR",
        code: 434,
        label: "Dollar",
    },
    LogicalKey {
        symbol: "KEY_EURO",
        code: 435,
        label: "Euro",
    },
    LogicalKey {
        symbol: "KEY_FRAMEBACK",
        code: 436,
        label: "Frameback",
    },
    LogicalKey {
        symbol: "KEY_FRAMEFORWARD",
        code: 437,
        label: "Frameforward",
    },
    LogicalKey {
        symbol: "KEY_CONTEXT_MENU",
        code: 438,
        label: "Context Menu",
    },
    LogicalKey {
        symbol: "KEY_MEDIA_REPEAT",
        code: 439,
        label: "Media Repeat",
    },
    LogicalKey {
        symbol: "KEY_10CHANNELSUP",
        code: 440,
        label: "10Channelsup",
    },
    LogicalKey {
        symbol: "KEY_10CHANNELSDOWN",
        code: 441,
        label: "10Channelsdown",
    },
    LogicalKey {
        symbol: "KEY_IMAGES",
        code: 442,
        label: "Images",
    },
    LogicalKey {
        symbol: "KEY_NOTIFICATION_CENTER",
        code: 444,
        label: "Notification Center",
    },
    LogicalKey {
        symbol: "KEY_PICKUP_PHONE",
        code: 445,
        label: "Pickup Phone",
    },
    LogicalKey {
        symbol: "KEY_HANGUP_PHONE",
        code: 446,
        label: "Hangup Phone",
    },
    LogicalKey {
        symbol: "KEY_LINK_PHONE",
        code: 447,
        label: "Link Phone",
    },
    LogicalKey {
        symbol: "KEY_DEL_EOL",
        code: 448,
        label: "Del Eol",
    },
    LogicalKey {
        symbol: "KEY_DEL_EOS",
        code: 449,
        label: "Del Eos",
    },
    LogicalKey {
        symbol: "KEY_INS_LINE",
        code: 450,
        label: "Ins Line",
    },
    LogicalKey {
        symbol: "KEY_DEL_LINE",
        code: 451,
        label: "Del Line",
    },
    LogicalKey {
        symbol: "KEY_FN",
        code: 464,
        label: "Fn",
    },
    LogicalKey {
        symbol: "KEY_FN_ESC",
        code: 465,
        label: "Fn Esc",
    },
    LogicalKey {
        symbol: "KEY_FN_F1",
        code: 466,
        label: "Fn F1",
    },
    LogicalKey {
        symbol: "KEY_FN_F2",
        code: 467,
        label: "Fn F2",
    },
    LogicalKey {
        symbol: "KEY_FN_F3",
        code: 468,
        label: "Fn F3",
    },
    LogicalKey {
        symbol: "KEY_FN_F4",
        code: 469,
        label: "Fn F4",
    },
    LogicalKey {
        symbol: "KEY_FN_F5",
        code: 470,
        label: "Fn F5",
    },
    LogicalKey {
        symbol: "KEY_FN_F6",
        code: 471,
        label: "Fn F6",
    },
    LogicalKey {
        symbol: "KEY_FN_F7",
        code: 472,
        label: "Fn F7",
    },
    LogicalKey {
        symbol: "KEY_FN_F8",
        code: 473,
        label: "Fn F8",
    },
    LogicalKey {
        symbol: "KEY_FN_F9",
        code: 474,
        label: "Fn F9",
    },
    LogicalKey {
        symbol: "KEY_FN_F10",
        code: 475,
        label: "Fn F10",
    },
    LogicalKey {
        symbol: "KEY_FN_F11",
        code: 476,
        label: "Fn F11",
    },
    LogicalKey {
        symbol: "KEY_FN_F12",
        code: 477,
        label: "Fn F12",
    },
    LogicalKey {
        symbol: "KEY_FN_1",
        code: 478,
        label: "Fn 1",
    },
    LogicalKey {
        symbol: "KEY_FN_2",
        code: 479,
        label: "Fn 2",
    },
    LogicalKey {
        symbol: "KEY_FN_D",
        code: 480,
        label: "Fn D",
    },
    LogicalKey {
        symbol: "KEY_FN_E",
        code: 481,
        label: "Fn E",
    },
    LogicalKey {
        symbol: "KEY_FN_F",
        code: 482,
        label: "Fn F",
    },
    LogicalKey {
        symbol: "KEY_FN_S",
        code: 483,
        label: "Fn S",
    },
    LogicalKey {
        symbol: "KEY_FN_B",
        code: 484,
        label: "Fn B",
    },
    LogicalKey {
        symbol: "KEY_FN_RIGHT_SHIFT",
        code: 485,
        label: "Fn Right Shift",
    },
    LogicalKey {
        symbol: "KEY_BRL_DOT1",
        code: 497,
        label: "Brl Dot1",
    },
    LogicalKey {
        symbol: "KEY_BRL_DOT2",
        code: 498,
        label: "Brl Dot2",
    },
    LogicalKey {
        symbol: "KEY_BRL_DOT3",
        code: 499,
        label: "Brl Dot3",
    },
    LogicalKey {
        symbol: "KEY_BRL_DOT4",
        code: 500,
        label: "Brl Dot4",
    },
    LogicalKey {
        symbol: "KEY_BRL_DOT5",
        code: 501,
        label: "Brl Dot5",
    },
    LogicalKey {
        symbol: "KEY_BRL_DOT6",
        code: 502,
        label: "Brl Dot6",
    },
    LogicalKey {
        symbol: "KEY_BRL_DOT7",
        code: 503,
        label: "Brl Dot7",
    },
    LogicalKey {
        symbol: "KEY_BRL_DOT8",
        code: 504,
        label: "Brl Dot8",
    },
    LogicalKey {
        symbol: "KEY_BRL_DOT9",
        code: 505,
        label: "Brl Dot9",
    },
    LogicalKey {
        symbol: "KEY_BRL_DOT10",
        code: 506,
        label: "Brl Dot10",
    },
    LogicalKey {
        symbol: "KEY_NUMERIC_0",
        code: 512,
        label: "Numeric 0",
    },
    LogicalKey {
        symbol: "KEY_NUMERIC_1",
        code: 513,
        label: "Numeric 1",
    },
    LogicalKey {
        symbol: "KEY_NUMERIC_2",
        code: 514,
        label: "Numeric 2",
    },
    LogicalKey {
        symbol: "KEY_NUMERIC_3",
        code: 515,
        label: "Numeric 3",
    },
    LogicalKey {
        symbol: "KEY_NUMERIC_4",
        code: 516,
        label: "Numeric 4",
    },
    LogicalKey {
        symbol: "KEY_NUMERIC_5",
        code: 517,
        label: "Numeric 5",
    },
    LogicalKey {
        symbol: "KEY_NUMERIC_6",
        code: 518,
        label: "Numeric 6",
    },
    LogicalKey {
        symbol: "KEY_NUMERIC_7",
        code: 519,
        label: "Numeric 7",
    },
    LogicalKey {
        symbol: "KEY_NUMERIC_8",
        code: 520,
        label: "Numeric 8",
    },
    LogicalKey {
        symbol: "KEY_NUMERIC_9",
        code: 521,
        label: "Numeric 9",
    },
    LogicalKey {
        symbol: "KEY_NUMERIC_STAR",
        code: 522,
        label: "Numeric Star",
    },
    LogicalKey {
        symbol: "KEY_NUMERIC_POUND",
        code: 523,
        label: "Numeric Pound",
    },
    LogicalKey {
        symbol: "KEY_NUMERIC_A",
        code: 524,
        label: "Numeric A",
    },
    LogicalKey {
        symbol: "KEY_NUMERIC_B",
        code: 525,
        label: "Numeric B",
    },
    LogicalKey {
        symbol: "KEY_NUMERIC_C",
        code: 526,
        label: "Numeric C",
    },
    LogicalKey {
        symbol: "KEY_NUMERIC_D",
        code: 527,
        label: "Numeric D",
    },
    LogicalKey {
        symbol: "KEY_CAMERA_FOCUS",
        code: 528,
        label: "Camera Focus",
    },
    LogicalKey {
        symbol: "KEY_WPS_BUTTON",
        code: 529,
        label: "Wps Button",
    },
    LogicalKey {
        symbol: "KEY_TOUCHPAD_TOGGLE",
        code: 530,
        label: "Touchpad Toggle",
    },
    LogicalKey {
        symbol: "KEY_TOUCHPAD_ON",
        code: 531,
        label: "Touchpad On",
    },
    LogicalKey {
        symbol: "KEY_TOUCHPAD_OFF",
        code: 532,
        label: "Touchpad Off",
    },
    LogicalKey {
        symbol: "KEY_CAMERA_ZOOMIN",
        code: 533,
        label: "Camera Zoomin",
    },
    LogicalKey {
        symbol: "KEY_CAMERA_ZOOMOUT",
        code: 534,
        label: "Camera Zoomout",
    },
    LogicalKey {
        symbol: "KEY_CAMERA_UP",
        code: 535,
        label: "Camera Up",
    },
    LogicalKey {
        symbol: "KEY_CAMERA_DOWN",
        code: 536,
        label: "Camera Down",
    },
    LogicalKey {
        symbol: "KEY_CAMERA_LEFT",
        code: 537,
        label: "Camera Left",
    },
    LogicalKey {
        symbol: "KEY_CAMERA_RIGHT",
        code: 538,
        label: "Camera Right",
    },
    LogicalKey {
        symbol: "KEY_ATTENDANT_ON",
        code: 539,
        label: "Attendant On",
    },
    LogicalKey {
        symbol: "KEY_ATTENDANT_OFF",
        code: 540,
        label: "Attendant Off",
    },
    LogicalKey {
        symbol: "KEY_ATTENDANT_TOGGLE",
        code: 541,
        label: "Attendant Toggle",
    },
    LogicalKey {
        symbol: "KEY_LIGHTS_TOGGLE",
        code: 542,
        label: "Lights Toggle",
    },
    LogicalKey {
        symbol: "KEY_ALS_TOGGLE",
        code: 560,
        label: "Als Toggle",
    },
    LogicalKey {
        symbol: "KEY_ROTATE_LOCK_TOGGLE",
        code: 561,
        label: "Rotate Lock Toggle",
    },
    LogicalKey {
        symbol: "KEY_REFRESH_RATE_TOGGLE",
        code: 562,
        label: "Refresh Rate Toggle",
    },
    LogicalKey {
        symbol: "KEY_BUTTONCONFIG",
        code: 576,
        label: "Buttonconfig",
    },
    LogicalKey {
        symbol: "KEY_TASKMANAGER",
        code: 577,
        label: "Taskmanager",
    },
    LogicalKey {
        symbol: "KEY_JOURNAL",
        code: 578,
        label: "Journal",
    },
    LogicalKey {
        symbol: "KEY_CONTROLPANEL",
        code: 579,
        label: "Controlpanel",
    },
    LogicalKey {
        symbol: "KEY_APPSELECT",
        code: 580,
        label: "Appselect",
    },
    LogicalKey {
        symbol: "KEY_SCREENSAVER",
        code: 581,
        label: "Screensaver",
    },
    LogicalKey {
        symbol: "KEY_VOICECOMMAND",
        code: 582,
        label: "Voicecommand",
    },
    LogicalKey {
        symbol: "KEY_ASSISTANT",
        code: 583,
        label: "Assistant",
    },
    LogicalKey {
        symbol: "KEY_KBD_LAYOUT_NEXT",
        code: 584,
        label: "Kbd Layout Next",
    },
    LogicalKey {
        symbol: "KEY_EMOJI_PICKER",
        code: 585,
        label: "Emoji Picker",
    },
    LogicalKey {
        symbol: "KEY_DICTATE",
        code: 586,
        label: "Dictate",
    },
    LogicalKey {
        symbol: "KEY_CAMERA_ACCESS_ENABLE",
        code: 587,
        label: "Camera Access Enable",
    },
    LogicalKey {
        symbol: "KEY_CAMERA_ACCESS_DISABLE",
        code: 588,
        label: "Camera Access Disable",
    },
    LogicalKey {
        symbol: "KEY_CAMERA_ACCESS_TOGGLE",
        code: 589,
        label: "Camera Access Toggle",
    },
    LogicalKey {
        symbol: "KEY_ACCESSIBILITY",
        code: 590,
        label: "Accessibility",
    },
    LogicalKey {
        symbol: "KEY_DO_NOT_DISTURB",
        code: 591,
        label: "Do Not Disturb",
    },
    LogicalKey {
        symbol: "KEY_BRIGHTNESS_MIN",
        code: 592,
        label: "Brightness Min",
    },
    LogicalKey {
        symbol: "KEY_BRIGHTNESS_MAX",
        code: 593,
        label: "Brightness Max",
    },
    LogicalKey {
        symbol: "KEY_KBDINPUTASSIST_PREV",
        code: 608,
        label: "Kbdinputassist Prev",
    },
    LogicalKey {
        symbol: "KEY_KBDINPUTASSIST_NEXT",
        code: 609,
        label: "Kbdinputassist Next",
    },
    LogicalKey {
        symbol: "KEY_KBDINPUTASSIST_PREVGROUP",
        code: 610,
        label: "Kbdinputassist Prevgroup",
    },
    LogicalKey {
        symbol: "KEY_KBDINPUTASSIST_NEXTGROUP",
        code: 611,
        label: "Kbdinputassist Nextgroup",
    },
    LogicalKey {
        symbol: "KEY_KBDINPUTASSIST_ACCEPT",
        code: 612,
        label: "Kbdinputassist Accept",
    },
    LogicalKey {
        symbol: "KEY_KBDINPUTASSIST_CANCEL",
        code: 613,
        label: "Kbdinputassist Cancel",
    },
    LogicalKey {
        symbol: "KEY_RIGHT_UP",
        code: 614,
        label: "Right Up",
    },
    LogicalKey {
        symbol: "KEY_RIGHT_DOWN",
        code: 615,
        label: "Right Down",
    },
    LogicalKey {
        symbol: "KEY_LEFT_UP",
        code: 616,
        label: "Left Up",
    },
    LogicalKey {
        symbol: "KEY_LEFT_DOWN",
        code: 617,
        label: "Left Down",
    },
    LogicalKey {
        symbol: "KEY_ROOT_MENU",
        code: 618,
        label: "Root Menu",
    },
    LogicalKey {
        symbol: "KEY_MEDIA_TOP_MENU",
        code: 619,
        label: "Media Top Menu",
    },
    LogicalKey {
        symbol: "KEY_NUMERIC_11",
        code: 620,
        label: "Numeric 11",
    },
    LogicalKey {
        symbol: "KEY_NUMERIC_12",
        code: 621,
        label: "Numeric 12",
    },
    LogicalKey {
        symbol: "KEY_AUDIO_DESC",
        code: 622,
        label: "Audio Desc",
    },
    LogicalKey {
        symbol: "KEY_3D_MODE",
        code: 623,
        label: "3D Mode",
    },
    LogicalKey {
        symbol: "KEY_NEXT_FAVORITE",
        code: 624,
        label: "Next Favorite",
    },
    LogicalKey {
        symbol: "KEY_STOP_RECORD",
        code: 625,
        label: "Stop Record",
    },
    LogicalKey {
        symbol: "KEY_PAUSE_RECORD",
        code: 626,
        label: "Pause Record",
    },
    LogicalKey {
        symbol: "KEY_VOD",
        code: 627,
        label: "Vod",
    },
    LogicalKey {
        symbol: "KEY_UNMUTE",
        code: 628,
        label: "Unmute",
    },
    LogicalKey {
        symbol: "KEY_FASTREVERSE",
        code: 629,
        label: "Fastreverse",
    },
    LogicalKey {
        symbol: "KEY_SLOWREVERSE",
        code: 630,
        label: "Slowreverse",
    },
    LogicalKey {
        symbol: "KEY_DATA",
        code: 631,
        label: "Data",
    },
    LogicalKey {
        symbol: "KEY_ONSCREEN_KEYBOARD",
        code: 632,
        label: "Onscreen Keyboard",
    },
    LogicalKey {
        symbol: "KEY_PRIVACY_SCREEN_TOGGLE",
        code: 633,
        label: "Privacy Screen Toggle",
    },
    LogicalKey {
        symbol: "KEY_SELECTIVE_SCREENSHOT",
        code: 634,
        label: "Selective Screenshot",
    },
    LogicalKey {
        symbol: "KEY_NEXT_ELEMENT",
        code: 635,
        label: "Next Element",
    },
    LogicalKey {
        symbol: "KEY_PREVIOUS_ELEMENT",
        code: 636,
        label: "Previous Element",
    },
    LogicalKey {
        symbol: "KEY_AUTOPILOT_ENGAGE_TOGGLE",
        code: 637,
        label: "Autopilot Engage Toggle",
    },
    LogicalKey {
        symbol: "KEY_MARK_WAYPOINT",
        code: 638,
        label: "Mark Waypoint",
    },
    LogicalKey {
        symbol: "KEY_SOS",
        code: 639,
        label: "Sos",
    },
    LogicalKey {
        symbol: "KEY_NAV_CHART",
        code: 640,
        label: "Nav Chart",
    },
    LogicalKey {
        symbol: "KEY_FISHING_CHART",
        code: 641,
        label: "Fishing Chart",
    },
    LogicalKey {
        symbol: "KEY_SINGLE_RANGE_RADAR",
        code: 642,
        label: "Single Range Radar",
    },
    LogicalKey {
        symbol: "KEY_DUAL_RANGE_RADAR",
        code: 643,
        label: "Dual Range Radar",
    },
    LogicalKey {
        symbol: "KEY_RADAR_OVERLAY",
        code: 644,
        label: "Radar Overlay",
    },
    LogicalKey {
        symbol: "KEY_TRADITIONAL_SONAR",
        code: 645,
        label: "Traditional Sonar",
    },
    LogicalKey {
        symbol: "KEY_CLEARVU_SONAR",
        code: 646,
        label: "Clearvu Sonar",
    },
    LogicalKey {
        symbol: "KEY_SIDEVU_SONAR",
        code: 647,
        label: "Sidevu Sonar",
    },
    LogicalKey {
        symbol: "KEY_NAV_INFO",
        code: 648,
        label: "Nav Info",
    },
    LogicalKey {
        symbol: "KEY_BRIGHTNESS_MENU",
        code: 649,
        label: "Brightness Menu",
    },
    LogicalKey {
        symbol: "KEY_MACRO1",
        code: 656,
        label: "Macro1",
    },
    LogicalKey {
        symbol: "KEY_MACRO2",
        code: 657,
        label: "Macro2",
    },
    LogicalKey {
        symbol: "KEY_MACRO3",
        code: 658,
        label: "Macro3",
    },
    LogicalKey {
        symbol: "KEY_MACRO4",
        code: 659,
        label: "Macro4",
    },
    LogicalKey {
        symbol: "KEY_MACRO5",
        code: 660,
        label: "Macro5",
    },
    LogicalKey {
        symbol: "KEY_MACRO6",
        code: 661,
        label: "Macro6",
    },
    LogicalKey {
        symbol: "KEY_MACRO7",
        code: 662,
        label: "Macro7",
    },
    LogicalKey {
        symbol: "KEY_MACRO8",
        code: 663,
        label: "Macro8",
    },
    LogicalKey {
        symbol: "KEY_MACRO9",
        code: 664,
        label: "Macro9",
    },
    LogicalKey {
        symbol: "KEY_MACRO10",
        code: 665,
        label: "Macro10",
    },
    LogicalKey {
        symbol: "KEY_MACRO11",
        code: 666,
        label: "Macro11",
    },
    LogicalKey {
        symbol: "KEY_MACRO12",
        code: 667,
        label: "Macro12",
    },
    LogicalKey {
        symbol: "KEY_MACRO13",
        code: 668,
        label: "Macro13",
    },
    LogicalKey {
        symbol: "KEY_MACRO14",
        code: 669,
        label: "Macro14",
    },
    LogicalKey {
        symbol: "KEY_MACRO15",
        code: 670,
        label: "Macro15",
    },
    LogicalKey {
        symbol: "KEY_MACRO16",
        code: 671,
        label: "Macro16",
    },
    LogicalKey {
        symbol: "KEY_MACRO17",
        code: 672,
        label: "Macro17",
    },
    LogicalKey {
        symbol: "KEY_MACRO18",
        code: 673,
        label: "Macro18",
    },
    LogicalKey {
        symbol: "KEY_MACRO19",
        code: 674,
        label: "Macro19",
    },
    LogicalKey {
        symbol: "KEY_MACRO20",
        code: 675,
        label: "Macro20",
    },
    LogicalKey {
        symbol: "KEY_MACRO21",
        code: 676,
        label: "Macro21",
    },
    LogicalKey {
        symbol: "KEY_MACRO22",
        code: 677,
        label: "Macro22",
    },
    LogicalKey {
        symbol: "KEY_MACRO23",
        code: 678,
        label: "Macro23",
    },
    LogicalKey {
        symbol: "KEY_MACRO24",
        code: 679,
        label: "Macro24",
    },
    LogicalKey {
        symbol: "KEY_MACRO25",
        code: 680,
        label: "Macro25",
    },
    LogicalKey {
        symbol: "KEY_MACRO26",
        code: 681,
        label: "Macro26",
    },
    LogicalKey {
        symbol: "KEY_MACRO27",
        code: 682,
        label: "Macro27",
    },
    LogicalKey {
        symbol: "KEY_MACRO28",
        code: 683,
        label: "Macro28",
    },
    LogicalKey {
        symbol: "KEY_MACRO29",
        code: 684,
        label: "Macro29",
    },
    LogicalKey {
        symbol: "KEY_MACRO30",
        code: 685,
        label: "Macro30",
    },
    LogicalKey {
        symbol: "KEY_MACRO_RECORD_START",
        code: 688,
        label: "Macro Record Start",
    },
    LogicalKey {
        symbol: "KEY_MACRO_RECORD_STOP",
        code: 689,
        label: "Macro Record Stop",
    },
    LogicalKey {
        symbol: "KEY_MACRO_PRESET_CYCLE",
        code: 690,
        label: "Macro Preset Cycle",
    },
    LogicalKey {
        symbol: "KEY_MACRO_PRESET1",
        code: 691,
        label: "Macro Preset1",
    },
    LogicalKey {
        symbol: "KEY_MACRO_PRESET2",
        code: 692,
        label: "Macro Preset2",
    },
    LogicalKey {
        symbol: "KEY_MACRO_PRESET3",
        code: 693,
        label: "Macro Preset3",
    },
    LogicalKey {
        symbol: "KEY_KBD_LCD_MENU1",
        code: 696,
        label: "Kbd Lcd Menu1",
    },
    LogicalKey {
        symbol: "KEY_KBD_LCD_MENU2",
        code: 697,
        label: "Kbd Lcd Menu2",
    },
    LogicalKey {
        symbol: "KEY_KBD_LCD_MENU3",
        code: 698,
        label: "Kbd Lcd Menu3",
    },
    LogicalKey {
        symbol: "KEY_KBD_LCD_MENU4",
        code: 699,
        label: "Kbd Lcd Menu4",
    },
    LogicalKey {
        symbol: "KEY_KBD_LCD_MENU5",
        code: 700,
        label: "Kbd Lcd Menu5",
    },
];
