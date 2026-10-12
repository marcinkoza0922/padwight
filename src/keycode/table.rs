//! Linux key and button codes (`input-event-codes.h`), by name and number, for platforms the
//! `evdev` crate doesn't build on. Generated from evdev 0.13's `scancodes.rs`, so names and
//! numbers are the same everywhere and configs and packs carry over unchanged.

use super::KeyCode;

impl KeyCode {
    pub const KEY_RESERVED: Self = Self(0);
    pub const KEY_ESC: Self = Self(1);
    pub const KEY_1: Self = Self(2);
    pub const KEY_2: Self = Self(3);
    pub const KEY_3: Self = Self(4);
    pub const KEY_4: Self = Self(5);
    pub const KEY_5: Self = Self(6);
    pub const KEY_6: Self = Self(7);
    pub const KEY_7: Self = Self(8);
    pub const KEY_8: Self = Self(9);
    pub const KEY_9: Self = Self(10);
    pub const KEY_0: Self = Self(11);
    pub const KEY_MINUS: Self = Self(12);
    pub const KEY_EQUAL: Self = Self(13);
    pub const KEY_BACKSPACE: Self = Self(14);
    pub const KEY_TAB: Self = Self(15);
    pub const KEY_Q: Self = Self(16);
    pub const KEY_W: Self = Self(17);
    pub const KEY_E: Self = Self(18);
    pub const KEY_R: Self = Self(19);
    pub const KEY_T: Self = Self(20);
    pub const KEY_Y: Self = Self(21);
    pub const KEY_U: Self = Self(22);
    pub const KEY_I: Self = Self(23);
    pub const KEY_O: Self = Self(24);
    pub const KEY_P: Self = Self(25);
    pub const KEY_LEFTBRACE: Self = Self(26);
    pub const KEY_RIGHTBRACE: Self = Self(27);
    pub const KEY_ENTER: Self = Self(28);
    pub const KEY_LEFTCTRL: Self = Self(29);
    pub const KEY_A: Self = Self(30);
    pub const KEY_S: Self = Self(31);
    pub const KEY_D: Self = Self(32);
    pub const KEY_F: Self = Self(33);
    pub const KEY_G: Self = Self(34);
    pub const KEY_H: Self = Self(35);
    pub const KEY_J: Self = Self(36);
    pub const KEY_K: Self = Self(37);
    pub const KEY_L: Self = Self(38);
    pub const KEY_SEMICOLON: Self = Self(39);
    pub const KEY_APOSTROPHE: Self = Self(40);
    pub const KEY_GRAVE: Self = Self(41);
    pub const KEY_LEFTSHIFT: Self = Self(42);
    pub const KEY_BACKSLASH: Self = Self(43);
    pub const KEY_Z: Self = Self(44);
    pub const KEY_X: Self = Self(45);
    pub const KEY_C: Self = Self(46);
    pub const KEY_V: Self = Self(47);
    pub const KEY_B: Self = Self(48);
    pub const KEY_N: Self = Self(49);
    pub const KEY_M: Self = Self(50);
    pub const KEY_COMMA: Self = Self(51);
    pub const KEY_DOT: Self = Self(52);
    pub const KEY_SLASH: Self = Self(53);
    pub const KEY_RIGHTSHIFT: Self = Self(54);
    pub const KEY_KPASTERISK: Self = Self(55);
    pub const KEY_LEFTALT: Self = Self(56);
    pub const KEY_SPACE: Self = Self(57);
    pub const KEY_CAPSLOCK: Self = Self(58);
    pub const KEY_F1: Self = Self(59);
    pub const KEY_F2: Self = Self(60);
    pub const KEY_F3: Self = Self(61);
    pub const KEY_F4: Self = Self(62);
    pub const KEY_F5: Self = Self(63);
    pub const KEY_F6: Self = Self(64);
    pub const KEY_F7: Self = Self(65);
    pub const KEY_F8: Self = Self(66);
    pub const KEY_F9: Self = Self(67);
    pub const KEY_F10: Self = Self(68);
    pub const KEY_NUMLOCK: Self = Self(69);
    pub const KEY_SCROLLLOCK: Self = Self(70);
    pub const KEY_KP7: Self = Self(71);
    pub const KEY_KP8: Self = Self(72);
    pub const KEY_KP9: Self = Self(73);
    pub const KEY_KPMINUS: Self = Self(74);
    pub const KEY_KP4: Self = Self(75);
    pub const KEY_KP5: Self = Self(76);
    pub const KEY_KP6: Self = Self(77);
    pub const KEY_KPPLUS: Self = Self(78);
    pub const KEY_KP1: Self = Self(79);
    pub const KEY_KP2: Self = Self(80);
    pub const KEY_KP3: Self = Self(81);
    pub const KEY_KP0: Self = Self(82);
    pub const KEY_KPDOT: Self = Self(83);
    pub const KEY_ZENKAKUHANKAKU: Self = Self(85);
    pub const KEY_102ND: Self = Self(86);
    pub const KEY_F11: Self = Self(87);
    pub const KEY_F12: Self = Self(88);
    pub const KEY_RO: Self = Self(89);
    pub const KEY_KATAKANA: Self = Self(90);
    pub const KEY_HIRAGANA: Self = Self(91);
    pub const KEY_HENKAN: Self = Self(92);
    pub const KEY_KATAKANAHIRAGANA: Self = Self(93);
    pub const KEY_MUHENKAN: Self = Self(94);
    pub const KEY_KPJPCOMMA: Self = Self(95);
    pub const KEY_KPENTER: Self = Self(96);
    pub const KEY_RIGHTCTRL: Self = Self(97);
    pub const KEY_KPSLASH: Self = Self(98);
    pub const KEY_SYSRQ: Self = Self(99);
    pub const KEY_RIGHTALT: Self = Self(100);
    pub const KEY_LINEFEED: Self = Self(101);
    pub const KEY_HOME: Self = Self(102);
    pub const KEY_UP: Self = Self(103);
    pub const KEY_PAGEUP: Self = Self(104);
    pub const KEY_LEFT: Self = Self(105);
    pub const KEY_RIGHT: Self = Self(106);
    pub const KEY_END: Self = Self(107);
    pub const KEY_DOWN: Self = Self(108);
    pub const KEY_PAGEDOWN: Self = Self(109);
    pub const KEY_INSERT: Self = Self(110);
    pub const KEY_DELETE: Self = Self(111);
    pub const KEY_MACRO: Self = Self(112);
    pub const KEY_MUTE: Self = Self(113);
    pub const KEY_VOLUMEDOWN: Self = Self(114);
    pub const KEY_VOLUMEUP: Self = Self(115);
    pub const KEY_POWER: Self = Self(116);
    pub const KEY_KPEQUAL: Self = Self(117);
    pub const KEY_KPPLUSMINUS: Self = Self(118);
    pub const KEY_PAUSE: Self = Self(119);
    pub const KEY_SCALE: Self = Self(120);
    pub const KEY_KPCOMMA: Self = Self(121);
    pub const KEY_HANGEUL: Self = Self(122);
    pub const KEY_HANJA: Self = Self(123);
    pub const KEY_YEN: Self = Self(124);
    pub const KEY_LEFTMETA: Self = Self(125);
    pub const KEY_RIGHTMETA: Self = Self(126);
    pub const KEY_COMPOSE: Self = Self(127);
    pub const KEY_STOP: Self = Self(128);
    pub const KEY_AGAIN: Self = Self(129);
    pub const KEY_PROPS: Self = Self(130);
    pub const KEY_UNDO: Self = Self(131);
    pub const KEY_FRONT: Self = Self(132);
    pub const KEY_COPY: Self = Self(133);
    pub const KEY_OPEN: Self = Self(134);
    pub const KEY_PASTE: Self = Self(135);
    pub const KEY_FIND: Self = Self(136);
    pub const KEY_CUT: Self = Self(137);
    pub const KEY_HELP: Self = Self(138);
    pub const KEY_MENU: Self = Self(139);
    pub const KEY_CALC: Self = Self(140);
    pub const KEY_SETUP: Self = Self(141);
    pub const KEY_SLEEP: Self = Self(142);
    pub const KEY_WAKEUP: Self = Self(143);
    pub const KEY_FILE: Self = Self(144);
    pub const KEY_SENDFILE: Self = Self(145);
    pub const KEY_DELETEFILE: Self = Self(146);
    pub const KEY_XFER: Self = Self(147);
    pub const KEY_PROG1: Self = Self(148);
    pub const KEY_PROG2: Self = Self(149);
    pub const KEY_WWW: Self = Self(150);
    pub const KEY_MSDOS: Self = Self(151);
    pub const KEY_COFFEE: Self = Self(152);
    pub const KEY_DIRECTION: Self = Self(153);
    pub const KEY_ROTATE_DISPLAY: Self = Self(153);
    pub const KEY_CYCLEWINDOWS: Self = Self(154);
    pub const KEY_MAIL: Self = Self(155);
    pub const KEY_BOOKMARKS: Self = Self(156);
    pub const KEY_COMPUTER: Self = Self(157);
    pub const KEY_BACK: Self = Self(158);
    pub const KEY_FORWARD: Self = Self(159);
    pub const KEY_CLOSECD: Self = Self(160);
    pub const KEY_EJECTCD: Self = Self(161);
    pub const KEY_EJECTCLOSECD: Self = Self(162);
    pub const KEY_NEXTSONG: Self = Self(163);
    pub const KEY_PLAYPAUSE: Self = Self(164);
    pub const KEY_PREVIOUSSONG: Self = Self(165);
    pub const KEY_STOPCD: Self = Self(166);
    pub const KEY_RECORD: Self = Self(167);
    pub const KEY_REWIND: Self = Self(168);
    pub const KEY_PHONE: Self = Self(169);
    pub const KEY_ISO: Self = Self(170);
    pub const KEY_CONFIG: Self = Self(171);
    pub const KEY_HOMEPAGE: Self = Self(172);
    pub const KEY_REFRESH: Self = Self(173);
    pub const KEY_EXIT: Self = Self(174);
    pub const KEY_MOVE: Self = Self(175);
    pub const KEY_EDIT: Self = Self(176);
    pub const KEY_SCROLLUP: Self = Self(177);
    pub const KEY_SCROLLDOWN: Self = Self(178);
    pub const KEY_KPLEFTPAREN: Self = Self(179);
    pub const KEY_KPRIGHTPAREN: Self = Self(180);
    pub const KEY_NEW: Self = Self(181);
    pub const KEY_REDO: Self = Self(182);
    pub const KEY_F13: Self = Self(183);
    pub const KEY_F14: Self = Self(184);
    pub const KEY_F15: Self = Self(185);
    pub const KEY_F16: Self = Self(186);
    pub const KEY_F17: Self = Self(187);
    pub const KEY_F18: Self = Self(188);
    pub const KEY_F19: Self = Self(189);
    pub const KEY_F20: Self = Self(190);
    pub const KEY_F21: Self = Self(191);
    pub const KEY_F22: Self = Self(192);
    pub const KEY_F23: Self = Self(193);
    pub const KEY_F24: Self = Self(194);
    pub const KEY_PLAYCD: Self = Self(200);
    pub const KEY_PAUSECD: Self = Self(201);
    pub const KEY_PROG3: Self = Self(202);
    pub const KEY_PROG4: Self = Self(203);
    pub const KEY_DASHBOARD: Self = Self(204);
    pub const KEY_SUSPEND: Self = Self(205);
    pub const KEY_CLOSE: Self = Self(206);
    pub const KEY_PLAY: Self = Self(207);
    pub const KEY_FASTFORWARD: Self = Self(208);
    pub const KEY_BASSBOOST: Self = Self(209);
    pub const KEY_PRINT: Self = Self(210);
    pub const KEY_HP: Self = Self(211);
    pub const KEY_CAMERA: Self = Self(212);
    pub const KEY_SOUND: Self = Self(213);
    pub const KEY_QUESTION: Self = Self(214);
    pub const KEY_EMAIL: Self = Self(215);
    pub const KEY_CHAT: Self = Self(216);
    pub const KEY_SEARCH: Self = Self(217);
    pub const KEY_CONNECT: Self = Self(218);
    pub const KEY_FINANCE: Self = Self(219);
    pub const KEY_SPORT: Self = Self(220);
    pub const KEY_SHOP: Self = Self(221);
    pub const KEY_ALTERASE: Self = Self(222);
    pub const KEY_CANCEL: Self = Self(223);
    pub const KEY_BRIGHTNESSDOWN: Self = Self(224);
    pub const KEY_BRIGHTNESSUP: Self = Self(225);
    pub const KEY_MEDIA: Self = Self(226);
    pub const KEY_SWITCHVIDEOMODE: Self = Self(227);
    pub const KEY_KBDILLUMTOGGLE: Self = Self(228);
    pub const KEY_KBDILLUMDOWN: Self = Self(229);
    pub const KEY_KBDILLUMUP: Self = Self(230);
    pub const KEY_SEND: Self = Self(231);
    pub const KEY_REPLY: Self = Self(232);
    pub const KEY_FORWARDMAIL: Self = Self(233);
    pub const KEY_SAVE: Self = Self(234);
    pub const KEY_DOCUMENTS: Self = Self(235);
    pub const KEY_BATTERY: Self = Self(236);
    pub const KEY_BLUETOOTH: Self = Self(237);
    pub const KEY_WLAN: Self = Self(238);
    pub const KEY_UWB: Self = Self(239);
    pub const KEY_UNKNOWN: Self = Self(240);
    pub const KEY_VIDEO_NEXT: Self = Self(241);
    pub const KEY_VIDEO_PREV: Self = Self(242);
    pub const KEY_BRIGHTNESS_CYCLE: Self = Self(243);
    pub const KEY_BRIGHTNESS_AUTO: Self = Self(244);
    pub const KEY_DISPLAY_OFF: Self = Self(245);
    pub const KEY_WWAN: Self = Self(246);
    pub const KEY_RFKILL: Self = Self(247);
    pub const KEY_MICMUTE: Self = Self(248);
    pub const BTN_0: Self = Self(0x100);
    pub const BTN_1: Self = Self(0x101);
    pub const BTN_2: Self = Self(0x102);
    pub const BTN_3: Self = Self(0x103);
    pub const BTN_4: Self = Self(0x104);
    pub const BTN_5: Self = Self(0x105);
    pub const BTN_6: Self = Self(0x106);
    pub const BTN_7: Self = Self(0x107);
    pub const BTN_8: Self = Self(0x108);
    pub const BTN_9: Self = Self(0x109);
    pub const BTN_LEFT: Self = Self(0x110);
    pub const BTN_RIGHT: Self = Self(0x111);
    pub const BTN_MIDDLE: Self = Self(0x112);
    pub const BTN_SIDE: Self = Self(0x113);
    pub const BTN_EXTRA: Self = Self(0x114);
    pub const BTN_FORWARD: Self = Self(0x115);
    pub const BTN_BACK: Self = Self(0x116);
    pub const BTN_TASK: Self = Self(0x117);
    pub const BTN_TRIGGER: Self = Self(0x120);
    pub const BTN_THUMB: Self = Self(0x121);
    pub const BTN_THUMB2: Self = Self(0x122);
    pub const BTN_TOP: Self = Self(0x123);
    pub const BTN_TOP2: Self = Self(0x124);
    pub const BTN_PINKIE: Self = Self(0x125);
    pub const BTN_BASE: Self = Self(0x126);
    pub const BTN_BASE2: Self = Self(0x127);
    pub const BTN_BASE3: Self = Self(0x128);
    pub const BTN_BASE4: Self = Self(0x129);
    pub const BTN_BASE5: Self = Self(0x12a);
    pub const BTN_BASE6: Self = Self(0x12b);
    pub const BTN_DEAD: Self = Self(0x12f);
    pub const BTN_SOUTH: Self = Self(0x130);
    pub const BTN_EAST: Self = Self(0x131);
    pub const BTN_C: Self = Self(0x132);
    pub const BTN_NORTH: Self = Self(0x133);
    pub const BTN_WEST: Self = Self(0x134);
    pub const BTN_Z: Self = Self(0x135);
    pub const BTN_TL: Self = Self(0x136);
    pub const BTN_TR: Self = Self(0x137);
    pub const BTN_TL2: Self = Self(0x138);
    pub const BTN_TR2: Self = Self(0x139);
    pub const BTN_SELECT: Self = Self(0x13a);
    pub const BTN_START: Self = Self(0x13b);
    pub const BTN_MODE: Self = Self(0x13c);
    pub const BTN_THUMBL: Self = Self(0x13d);
    pub const BTN_THUMBR: Self = Self(0x13e);
    pub const BTN_TOOL_PEN: Self = Self(0x140);
    pub const BTN_TOOL_RUBBER: Self = Self(0x141);
    pub const BTN_TOOL_BRUSH: Self = Self(0x142);
    pub const BTN_TOOL_PENCIL: Self = Self(0x143);
    pub const BTN_TOOL_AIRBRUSH: Self = Self(0x144);
    pub const BTN_TOOL_FINGER: Self = Self(0x145);
    pub const BTN_TOOL_MOUSE: Self = Self(0x146);
    pub const BTN_TOOL_LENS: Self = Self(0x147);
    pub const BTN_TOOL_QUINTTAP: Self = Self(0x148);
    pub const BTN_TOUCH: Self = Self(0x14a);
    pub const BTN_STYLUS: Self = Self(0x14b);
    pub const BTN_STYLUS2: Self = Self(0x14c);
    pub const BTN_TOOL_DOUBLETAP: Self = Self(0x14d);
    pub const BTN_TOOL_TRIPLETAP: Self = Self(0x14e);
    pub const BTN_TOOL_QUADTAP: Self = Self(0x14f);
    pub const BTN_GEAR_DOWN: Self = Self(0x150);
    pub const BTN_GEAR_UP: Self = Self(0x151);
    pub const KEY_OK: Self = Self(0x160);
    pub const KEY_SELECT: Self = Self(0x161);
    pub const KEY_GOTO: Self = Self(0x162);
    pub const KEY_CLEAR: Self = Self(0x163);
    pub const KEY_POWER2: Self = Self(0x164);
    pub const KEY_OPTION: Self = Self(0x165);
    pub const KEY_INFO: Self = Self(0x166);
    pub const KEY_TIME: Self = Self(0x167);
    pub const KEY_VENDOR: Self = Self(0x168);
    pub const KEY_ARCHIVE: Self = Self(0x169);
    pub const KEY_PROGRAM: Self = Self(0x16a);
    pub const KEY_CHANNEL: Self = Self(0x16b);
    pub const KEY_FAVORITES: Self = Self(0x16c);
    pub const KEY_EPG: Self = Self(0x16d);
    pub const KEY_PVR: Self = Self(0x16e);
    pub const KEY_MHP: Self = Self(0x16f);
    pub const KEY_LANGUAGE: Self = Self(0x170);
    pub const KEY_TITLE: Self = Self(0x171);
    pub const KEY_SUBTITLE: Self = Self(0x172);
    pub const KEY_ANGLE: Self = Self(0x173);
    pub const KEY_ZOOM: Self = Self(0x174);
    pub const KEY_FULL_SCREEN: Self = Self(0x174);
    pub const KEY_MODE: Self = Self(0x175);
    pub const KEY_KEYBOARD: Self = Self(0x176);
    pub const KEY_SCREEN: Self = Self(0x177);
    pub const KEY_PC: Self = Self(0x178);
    pub const KEY_TV: Self = Self(0x179);
    pub const KEY_TV2: Self = Self(0x17a);
    pub const KEY_VCR: Self = Self(0x17b);
    pub const KEY_VCR2: Self = Self(0x17c);
    pub const KEY_SAT: Self = Self(0x17d);
    pub const KEY_SAT2: Self = Self(0x17e);
    pub const KEY_CD: Self = Self(0x17f);
    pub const KEY_TAPE: Self = Self(0x180);
    pub const KEY_RADIO: Self = Self(0x181);
    pub const KEY_TUNER: Self = Self(0x182);
    pub const KEY_PLAYER: Self = Self(0x183);
    pub const KEY_TEXT: Self = Self(0x184);
    pub const KEY_DVD: Self = Self(0x185);
    pub const KEY_AUX: Self = Self(0x186);
    pub const KEY_MP3: Self = Self(0x187);
    pub const KEY_AUDIO: Self = Self(0x188);
    pub const KEY_VIDEO: Self = Self(0x189);
    pub const KEY_DIRECTORY: Self = Self(0x18a);
    pub const KEY_LIST: Self = Self(0x18b);
    pub const KEY_MEMO: Self = Self(0x18c);
    pub const KEY_CALENDAR: Self = Self(0x18d);
    pub const KEY_RED: Self = Self(0x18e);
    pub const KEY_GREEN: Self = Self(0x18f);
    pub const KEY_YELLOW: Self = Self(0x190);
    pub const KEY_BLUE: Self = Self(0x191);
    pub const KEY_CHANNELUP: Self = Self(0x192);
    pub const KEY_CHANNELDOWN: Self = Self(0x193);
    pub const KEY_FIRST: Self = Self(0x194);
    pub const KEY_LAST: Self = Self(0x195);
    pub const KEY_AB: Self = Self(0x196);
    pub const KEY_NEXT: Self = Self(0x197);
    pub const KEY_RESTART: Self = Self(0x198);
    pub const KEY_SLOW: Self = Self(0x199);
    pub const KEY_SHUFFLE: Self = Self(0x19a);
    pub const KEY_BREAK: Self = Self(0x19b);
    pub const KEY_PREVIOUS: Self = Self(0x19c);
    pub const KEY_DIGITS: Self = Self(0x19d);
    pub const KEY_TEEN: Self = Self(0x19e);
    pub const KEY_TWEN: Self = Self(0x19f);
    pub const KEY_VIDEOPHONE: Self = Self(0x1a0);
    pub const KEY_GAMES: Self = Self(0x1a1);
    pub const KEY_ZOOMIN: Self = Self(0x1a2);
    pub const KEY_ZOOMOUT: Self = Self(0x1a3);
    pub const KEY_ZOOMRESET: Self = Self(0x1a4);
    pub const KEY_WORDPROCESSOR: Self = Self(0x1a5);
    pub const KEY_EDITOR: Self = Self(0x1a6);
    pub const KEY_SPREADSHEET: Self = Self(0x1a7);
    pub const KEY_GRAPHICSEDITOR: Self = Self(0x1a8);
    pub const KEY_PRESENTATION: Self = Self(0x1a9);
    pub const KEY_DATABASE: Self = Self(0x1aa);
    pub const KEY_NEWS: Self = Self(0x1ab);
    pub const KEY_VOICEMAIL: Self = Self(0x1ac);
    pub const KEY_ADDRESSBOOK: Self = Self(0x1ad);
    pub const KEY_MESSENGER: Self = Self(0x1ae);
    pub const KEY_DISPLAYTOGGLE: Self = Self(0x1af);
    pub const KEY_SPELLCHECK: Self = Self(0x1b0);
    pub const KEY_LOGOFF: Self = Self(0x1b1);
    pub const KEY_DOLLAR: Self = Self(0x1b2);
    pub const KEY_EURO: Self = Self(0x1b3);
    pub const KEY_FRAMEBACK: Self = Self(0x1b4);
    pub const KEY_FRAMEFORWARD: Self = Self(0x1b5);
    pub const KEY_CONTEXT_MENU: Self = Self(0x1b6);
    pub const KEY_MEDIA_REPEAT: Self = Self(0x1b7);
    pub const KEY_10CHANNELSUP: Self = Self(0x1b8);
    pub const KEY_10CHANNELSDOWN: Self = Self(0x1b9);
    pub const KEY_IMAGES: Self = Self(0x1ba);
    pub const KEY_PICKUP_PHONE: Self = Self(0x1bd);
    pub const KEY_HANGUP_PHONE: Self = Self(0x1be);
    pub const KEY_DEL_EOL: Self = Self(0x1c0);
    pub const KEY_DEL_EOS: Self = Self(0x1c1);
    pub const KEY_INS_LINE: Self = Self(0x1c2);
    pub const KEY_DEL_LINE: Self = Self(0x1c3);
    pub const KEY_FN: Self = Self(0x1d0);
    pub const KEY_FN_ESC: Self = Self(0x1d1);
    pub const KEY_FN_F1: Self = Self(0x1d2);
    pub const KEY_FN_F2: Self = Self(0x1d3);
    pub const KEY_FN_F3: Self = Self(0x1d4);
    pub const KEY_FN_F4: Self = Self(0x1d5);
    pub const KEY_FN_F5: Self = Self(0x1d6);
    pub const KEY_FN_F6: Self = Self(0x1d7);
    pub const KEY_FN_F7: Self = Self(0x1d8);
    pub const KEY_FN_F8: Self = Self(0x1d9);
    pub const KEY_FN_F9: Self = Self(0x1da);
    pub const KEY_FN_F10: Self = Self(0x1db);
    pub const KEY_FN_F11: Self = Self(0x1dc);
    pub const KEY_FN_F12: Self = Self(0x1dd);
    pub const KEY_FN_1: Self = Self(0x1de);
    pub const KEY_FN_2: Self = Self(0x1df);
    pub const KEY_FN_D: Self = Self(0x1e0);
    pub const KEY_FN_E: Self = Self(0x1e1);
    pub const KEY_FN_F: Self = Self(0x1e2);
    pub const KEY_FN_S: Self = Self(0x1e3);
    pub const KEY_FN_B: Self = Self(0x1e4);
    pub const KEY_BRL_DOT1: Self = Self(0x1f1);
    pub const KEY_BRL_DOT2: Self = Self(0x1f2);
    pub const KEY_BRL_DOT3: Self = Self(0x1f3);
    pub const KEY_BRL_DOT4: Self = Self(0x1f4);
    pub const KEY_BRL_DOT5: Self = Self(0x1f5);
    pub const KEY_BRL_DOT6: Self = Self(0x1f6);
    pub const KEY_BRL_DOT7: Self = Self(0x1f7);
    pub const KEY_BRL_DOT8: Self = Self(0x1f8);
    pub const KEY_BRL_DOT9: Self = Self(0x1f9);
    pub const KEY_BRL_DOT10: Self = Self(0x1fa);
    pub const KEY_NUMERIC_0: Self = Self(0x200);
    pub const KEY_NUMERIC_1: Self = Self(0x201);
    pub const KEY_NUMERIC_2: Self = Self(0x202);
    pub const KEY_NUMERIC_3: Self = Self(0x203);
    pub const KEY_NUMERIC_4: Self = Self(0x204);
    pub const KEY_NUMERIC_5: Self = Self(0x205);
    pub const KEY_NUMERIC_6: Self = Self(0x206);
    pub const KEY_NUMERIC_7: Self = Self(0x207);
    pub const KEY_NUMERIC_8: Self = Self(0x208);
    pub const KEY_NUMERIC_9: Self = Self(0x209);
    pub const KEY_NUMERIC_STAR: Self = Self(0x20a);
    pub const KEY_NUMERIC_POUND: Self = Self(0x20b);
    pub const KEY_NUMERIC_A: Self = Self(0x20c);
    pub const KEY_NUMERIC_B: Self = Self(0x20d);
    pub const KEY_NUMERIC_C: Self = Self(0x20e);
    pub const KEY_NUMERIC_D: Self = Self(0x20f);
    pub const KEY_CAMERA_FOCUS: Self = Self(0x210);
    pub const KEY_WPS_BUTTON: Self = Self(0x211);
    pub const KEY_TOUCHPAD_TOGGLE: Self = Self(0x212);
    pub const KEY_TOUCHPAD_ON: Self = Self(0x213);
    pub const KEY_TOUCHPAD_OFF: Self = Self(0x214);
    pub const KEY_CAMERA_ZOOMIN: Self = Self(0x215);
    pub const KEY_CAMERA_ZOOMOUT: Self = Self(0x216);
    pub const KEY_CAMERA_UP: Self = Self(0x217);
    pub const KEY_CAMERA_DOWN: Self = Self(0x218);
    pub const KEY_CAMERA_LEFT: Self = Self(0x219);
    pub const KEY_CAMERA_RIGHT: Self = Self(0x21a);
    pub const KEY_ATTENDANT_ON: Self = Self(0x21b);
    pub const KEY_ATTENDANT_OFF: Self = Self(0x21c);
    pub const KEY_ATTENDANT_TOGGLE: Self = Self(0x21d);
    pub const KEY_LIGHTS_TOGGLE: Self = Self(0x21e);
    pub const BTN_DPAD_UP: Self = Self(0x220);
    pub const BTN_DPAD_DOWN: Self = Self(0x221);
    pub const BTN_DPAD_LEFT: Self = Self(0x222);
    pub const BTN_DPAD_RIGHT: Self = Self(0x223);
    pub const KEY_ALS_TOGGLE: Self = Self(0x230);
    pub const KEY_BUTTONCONFIG: Self = Self(0x240);
    pub const KEY_TASKMANAGER: Self = Self(0x241);
    pub const KEY_JOURNAL: Self = Self(0x242);
    pub const KEY_CONTROLPANEL: Self = Self(0x243);
    pub const KEY_APPSELECT: Self = Self(0x244);
    pub const KEY_SCREENSAVER: Self = Self(0x245);
    pub const KEY_VOICECOMMAND: Self = Self(0x246);
    pub const KEY_ASSISTANT: Self = Self(0x247);
    pub const KEY_KBD_LAYOUT_NEXT: Self = Self(0x248);
    pub const KEY_BRIGHTNESS_MIN: Self = Self(0x250);
    pub const KEY_BRIGHTNESS_MAX: Self = Self(0x251);
    pub const KEY_KBDINPUTASSIST_PREV: Self = Self(0x260);
    pub const KEY_KBDINPUTASSIST_NEXT: Self = Self(0x261);
    pub const KEY_KBDINPUTASSIST_PREVGROUP: Self = Self(0x262);
    pub const KEY_KBDINPUTASSIST_NEXTGROUP: Self = Self(0x263);
    pub const KEY_KBDINPUTASSIST_ACCEPT: Self = Self(0x264);
    pub const KEY_KBDINPUTASSIST_CANCEL: Self = Self(0x265);
    pub const KEY_RIGHT_UP: Self = Self(0x266);
    pub const KEY_RIGHT_DOWN: Self = Self(0x267);
    pub const KEY_LEFT_UP: Self = Self(0x268);
    pub const KEY_LEFT_DOWN: Self = Self(0x269);
    pub const KEY_ROOT_MENU: Self = Self(0x26a);
    pub const KEY_MEDIA_TOP_MENU: Self = Self(0x26b);
    pub const KEY_NUMERIC_11: Self = Self(0x26c);
    pub const KEY_NUMERIC_12: Self = Self(0x26d);
    pub const KEY_AUDIO_DESC: Self = Self(0x26e);
    pub const KEY_3D_MODE: Self = Self(0x26f);
    pub const KEY_NEXT_FAVORITE: Self = Self(0x270);
    pub const KEY_STOP_RECORD: Self = Self(0x271);
    pub const KEY_PAUSE_RECORD: Self = Self(0x272);
    pub const KEY_VOD: Self = Self(0x273);
    pub const KEY_UNMUTE: Self = Self(0x274);
    pub const KEY_FASTREVERSE: Self = Self(0x275);
    pub const KEY_SLOWREVERSE: Self = Self(0x276);
    pub const KEY_DATA: Self = Self(0x277);
    pub const KEY_ONSCREEN_KEYBOARD: Self = Self(0x278);
    pub const KEY_PRIVACY_SCREEN_TOGGLE: Self = Self(0x279);
    pub const KEY_SELECTIVE_SCREENSHOT: Self = Self(0x27a);
    pub const BTN_TRIGGER_HAPPY1: Self = Self(0x2c0);
    pub const BTN_TRIGGER_HAPPY2: Self = Self(0x2c1);
    pub const BTN_TRIGGER_HAPPY3: Self = Self(0x2c2);
    pub const BTN_TRIGGER_HAPPY4: Self = Self(0x2c3);
    pub const BTN_TRIGGER_HAPPY5: Self = Self(0x2c4);
    pub const BTN_TRIGGER_HAPPY6: Self = Self(0x2c5);
    pub const BTN_TRIGGER_HAPPY7: Self = Self(0x2c6);
    pub const BTN_TRIGGER_HAPPY8: Self = Self(0x2c7);
    pub const BTN_TRIGGER_HAPPY9: Self = Self(0x2c8);
    pub const BTN_TRIGGER_HAPPY10: Self = Self(0x2c9);
    pub const BTN_TRIGGER_HAPPY11: Self = Self(0x2ca);
    pub const BTN_TRIGGER_HAPPY12: Self = Self(0x2cb);
    pub const BTN_TRIGGER_HAPPY13: Self = Self(0x2cc);
    pub const BTN_TRIGGER_HAPPY14: Self = Self(0x2cd);
    pub const BTN_TRIGGER_HAPPY15: Self = Self(0x2ce);
    pub const BTN_TRIGGER_HAPPY16: Self = Self(0x2cf);
    pub const BTN_TRIGGER_HAPPY17: Self = Self(0x2d0);
    pub const BTN_TRIGGER_HAPPY18: Self = Self(0x2d1);
    pub const BTN_TRIGGER_HAPPY19: Self = Self(0x2d2);
    pub const BTN_TRIGGER_HAPPY20: Self = Self(0x2d3);
    pub const BTN_TRIGGER_HAPPY21: Self = Self(0x2d4);
    pub const BTN_TRIGGER_HAPPY22: Self = Self(0x2d5);
    pub const BTN_TRIGGER_HAPPY23: Self = Self(0x2d6);
    pub const BTN_TRIGGER_HAPPY24: Self = Self(0x2d7);
    pub const BTN_TRIGGER_HAPPY25: Self = Self(0x2d8);
    pub const BTN_TRIGGER_HAPPY26: Self = Self(0x2d9);
    pub const BTN_TRIGGER_HAPPY27: Self = Self(0x2da);
    pub const BTN_TRIGGER_HAPPY28: Self = Self(0x2db);
    pub const BTN_TRIGGER_HAPPY29: Self = Self(0x2dc);
    pub const BTN_TRIGGER_HAPPY30: Self = Self(0x2dd);
    pub const BTN_TRIGGER_HAPPY31: Self = Self(0x2de);
    pub const BTN_TRIGGER_HAPPY32: Self = Self(0x2df);
    pub const BTN_TRIGGER_HAPPY33: Self = Self(0x2e0);
    pub const BTN_TRIGGER_HAPPY34: Self = Self(0x2e1);
    pub const BTN_TRIGGER_HAPPY35: Self = Self(0x2e2);
    pub const BTN_TRIGGER_HAPPY36: Self = Self(0x2e3);
    pub const BTN_TRIGGER_HAPPY37: Self = Self(0x2e4);
    pub const BTN_TRIGGER_HAPPY38: Self = Self(0x2e5);
    pub const BTN_TRIGGER_HAPPY39: Self = Self(0x2e6);
    pub const BTN_TRIGGER_HAPPY40: Self = Self(0x2e7);

    /// Every name, in table order: where two names share a number, the first is the one shown.
    pub(super) const NAMES: &'static [(&'static str, KeyCode)] = &[
        ("KEY_RESERVED", Self::KEY_RESERVED),
        ("KEY_ESC", Self::KEY_ESC),
        ("KEY_1", Self::KEY_1),
        ("KEY_2", Self::KEY_2),
        ("KEY_3", Self::KEY_3),
        ("KEY_4", Self::KEY_4),
        ("KEY_5", Self::KEY_5),
        ("KEY_6", Self::KEY_6),
        ("KEY_7", Self::KEY_7),
        ("KEY_8", Self::KEY_8),
        ("KEY_9", Self::KEY_9),
        ("KEY_0", Self::KEY_0),
        ("KEY_MINUS", Self::KEY_MINUS),
        ("KEY_EQUAL", Self::KEY_EQUAL),
        ("KEY_BACKSPACE", Self::KEY_BACKSPACE),
        ("KEY_TAB", Self::KEY_TAB),
        ("KEY_Q", Self::KEY_Q),
        ("KEY_W", Self::KEY_W),
        ("KEY_E", Self::KEY_E),
        ("KEY_R", Self::KEY_R),
        ("KEY_T", Self::KEY_T),
        ("KEY_Y", Self::KEY_Y),
        ("KEY_U", Self::KEY_U),
        ("KEY_I", Self::KEY_I),
        ("KEY_O", Self::KEY_O),
        ("KEY_P", Self::KEY_P),
        ("KEY_LEFTBRACE", Self::KEY_LEFTBRACE),
        ("KEY_RIGHTBRACE", Self::KEY_RIGHTBRACE),
        ("KEY_ENTER", Self::KEY_ENTER),
        ("KEY_LEFTCTRL", Self::KEY_LEFTCTRL),
        ("KEY_A", Self::KEY_A),
        ("KEY_S", Self::KEY_S),
        ("KEY_D", Self::KEY_D),
        ("KEY_F", Self::KEY_F),
        ("KEY_G", Self::KEY_G),
        ("KEY_H", Self::KEY_H),
        ("KEY_J", Self::KEY_J),
        ("KEY_K", Self::KEY_K),
        ("KEY_L", Self::KEY_L),
        ("KEY_SEMICOLON", Self::KEY_SEMICOLON),
        ("KEY_APOSTROPHE", Self::KEY_APOSTROPHE),
        ("KEY_GRAVE", Self::KEY_GRAVE),
        ("KEY_LEFTSHIFT", Self::KEY_LEFTSHIFT),
        ("KEY_BACKSLASH", Self::KEY_BACKSLASH),
        ("KEY_Z", Self::KEY_Z),
        ("KEY_X", Self::KEY_X),
        ("KEY_C", Self::KEY_C),
        ("KEY_V", Self::KEY_V),
        ("KEY_B", Self::KEY_B),
        ("KEY_N", Self::KEY_N),
        ("KEY_M", Self::KEY_M),
        ("KEY_COMMA", Self::KEY_COMMA),
        ("KEY_DOT", Self::KEY_DOT),
        ("KEY_SLASH", Self::KEY_SLASH),
        ("KEY_RIGHTSHIFT", Self::KEY_RIGHTSHIFT),
        ("KEY_KPASTERISK", Self::KEY_KPASTERISK),
        ("KEY_LEFTALT", Self::KEY_LEFTALT),
        ("KEY_SPACE", Self::KEY_SPACE),
        ("KEY_CAPSLOCK", Self::KEY_CAPSLOCK),
        ("KEY_F1", Self::KEY_F1),
        ("KEY_F2", Self::KEY_F2),
        ("KEY_F3", Self::KEY_F3),
        ("KEY_F4", Self::KEY_F4),
        ("KEY_F5", Self::KEY_F5),
        ("KEY_F6", Self::KEY_F6),
        ("KEY_F7", Self::KEY_F7),
        ("KEY_F8", Self::KEY_F8),
        ("KEY_F9", Self::KEY_F9),
        ("KEY_F10", Self::KEY_F10),
        ("KEY_NUMLOCK", Self::KEY_NUMLOCK),
        ("KEY_SCROLLLOCK", Self::KEY_SCROLLLOCK),
        ("KEY_KP7", Self::KEY_KP7),
        ("KEY_KP8", Self::KEY_KP8),
        ("KEY_KP9", Self::KEY_KP9),
        ("KEY_KPMINUS", Self::KEY_KPMINUS),
        ("KEY_KP4", Self::KEY_KP4),
        ("KEY_KP5", Self::KEY_KP5),
        ("KEY_KP6", Self::KEY_KP6),
        ("KEY_KPPLUS", Self::KEY_KPPLUS),
        ("KEY_KP1", Self::KEY_KP1),
        ("KEY_KP2", Self::KEY_KP2),
        ("KEY_KP3", Self::KEY_KP3),
        ("KEY_KP0", Self::KEY_KP0),
        ("KEY_KPDOT", Self::KEY_KPDOT),
        ("KEY_ZENKAKUHANKAKU", Self::KEY_ZENKAKUHANKAKU),
        ("KEY_102ND", Self::KEY_102ND),
        ("KEY_F11", Self::KEY_F11),
        ("KEY_F12", Self::KEY_F12),
        ("KEY_RO", Self::KEY_RO),
        ("KEY_KATAKANA", Self::KEY_KATAKANA),
        ("KEY_HIRAGANA", Self::KEY_HIRAGANA),
        ("KEY_HENKAN", Self::KEY_HENKAN),
        ("KEY_KATAKANAHIRAGANA", Self::KEY_KATAKANAHIRAGANA),
        ("KEY_MUHENKAN", Self::KEY_MUHENKAN),
        ("KEY_KPJPCOMMA", Self::KEY_KPJPCOMMA),
        ("KEY_KPENTER", Self::KEY_KPENTER),
        ("KEY_RIGHTCTRL", Self::KEY_RIGHTCTRL),
        ("KEY_KPSLASH", Self::KEY_KPSLASH),
        ("KEY_SYSRQ", Self::KEY_SYSRQ),
        ("KEY_RIGHTALT", Self::KEY_RIGHTALT),
        ("KEY_LINEFEED", Self::KEY_LINEFEED),
        ("KEY_HOME", Self::KEY_HOME),
        ("KEY_UP", Self::KEY_UP),
        ("KEY_PAGEUP", Self::KEY_PAGEUP),
        ("KEY_LEFT", Self::KEY_LEFT),
        ("KEY_RIGHT", Self::KEY_RIGHT),
        ("KEY_END", Self::KEY_END),
        ("KEY_DOWN", Self::KEY_DOWN),
        ("KEY_PAGEDOWN", Self::KEY_PAGEDOWN),
        ("KEY_INSERT", Self::KEY_INSERT),
        ("KEY_DELETE", Self::KEY_DELETE),
        ("KEY_MACRO", Self::KEY_MACRO),
        ("KEY_MUTE", Self::KEY_MUTE),
        ("KEY_VOLUMEDOWN", Self::KEY_VOLUMEDOWN),
        ("KEY_VOLUMEUP", Self::KEY_VOLUMEUP),
        ("KEY_POWER", Self::KEY_POWER),
        ("KEY_KPEQUAL", Self::KEY_KPEQUAL),
        ("KEY_KPPLUSMINUS", Self::KEY_KPPLUSMINUS),
        ("KEY_PAUSE", Self::KEY_PAUSE),
        ("KEY_SCALE", Self::KEY_SCALE),
        ("KEY_KPCOMMA", Self::KEY_KPCOMMA),
        ("KEY_HANGEUL", Self::KEY_HANGEUL),
        ("KEY_HANJA", Self::KEY_HANJA),
        ("KEY_YEN", Self::KEY_YEN),
        ("KEY_LEFTMETA", Self::KEY_LEFTMETA),
        ("KEY_RIGHTMETA", Self::KEY_RIGHTMETA),
        ("KEY_COMPOSE", Self::KEY_COMPOSE),
        ("KEY_STOP", Self::KEY_STOP),
        ("KEY_AGAIN", Self::KEY_AGAIN),
        ("KEY_PROPS", Self::KEY_PROPS),
        ("KEY_UNDO", Self::KEY_UNDO),
        ("KEY_FRONT", Self::KEY_FRONT),
        ("KEY_COPY", Self::KEY_COPY),
        ("KEY_OPEN", Self::KEY_OPEN),
        ("KEY_PASTE", Self::KEY_PASTE),
        ("KEY_FIND", Self::KEY_FIND),
        ("KEY_CUT", Self::KEY_CUT),
        ("KEY_HELP", Self::KEY_HELP),
        ("KEY_MENU", Self::KEY_MENU),
        ("KEY_CALC", Self::KEY_CALC),
        ("KEY_SETUP", Self::KEY_SETUP),
        ("KEY_SLEEP", Self::KEY_SLEEP),
        ("KEY_WAKEUP", Self::KEY_WAKEUP),
        ("KEY_FILE", Self::KEY_FILE),
        ("KEY_SENDFILE", Self::KEY_SENDFILE),
        ("KEY_DELETEFILE", Self::KEY_DELETEFILE),
        ("KEY_XFER", Self::KEY_XFER),
        ("KEY_PROG1", Self::KEY_PROG1),
        ("KEY_PROG2", Self::KEY_PROG2),
        ("KEY_WWW", Self::KEY_WWW),
        ("KEY_MSDOS", Self::KEY_MSDOS),
        ("KEY_COFFEE", Self::KEY_COFFEE),
        ("KEY_DIRECTION", Self::KEY_DIRECTION),
        ("KEY_ROTATE_DISPLAY", Self::KEY_ROTATE_DISPLAY),
        ("KEY_CYCLEWINDOWS", Self::KEY_CYCLEWINDOWS),
        ("KEY_MAIL", Self::KEY_MAIL),
        ("KEY_BOOKMARKS", Self::KEY_BOOKMARKS),
        ("KEY_COMPUTER", Self::KEY_COMPUTER),
        ("KEY_BACK", Self::KEY_BACK),
        ("KEY_FORWARD", Self::KEY_FORWARD),
        ("KEY_CLOSECD", Self::KEY_CLOSECD),
        ("KEY_EJECTCD", Self::KEY_EJECTCD),
        ("KEY_EJECTCLOSECD", Self::KEY_EJECTCLOSECD),
        ("KEY_NEXTSONG", Self::KEY_NEXTSONG),
        ("KEY_PLAYPAUSE", Self::KEY_PLAYPAUSE),
        ("KEY_PREVIOUSSONG", Self::KEY_PREVIOUSSONG),
        ("KEY_STOPCD", Self::KEY_STOPCD),
        ("KEY_RECORD", Self::KEY_RECORD),
        ("KEY_REWIND", Self::KEY_REWIND),
        ("KEY_PHONE", Self::KEY_PHONE),
        ("KEY_ISO", Self::KEY_ISO),
        ("KEY_CONFIG", Self::KEY_CONFIG),
        ("KEY_HOMEPAGE", Self::KEY_HOMEPAGE),
        ("KEY_REFRESH", Self::KEY_REFRESH),
        ("KEY_EXIT", Self::KEY_EXIT),
        ("KEY_MOVE", Self::KEY_MOVE),
        ("KEY_EDIT", Self::KEY_EDIT),
        ("KEY_SCROLLUP", Self::KEY_SCROLLUP),
        ("KEY_SCROLLDOWN", Self::KEY_SCROLLDOWN),
        ("KEY_KPLEFTPAREN", Self::KEY_KPLEFTPAREN),
        ("KEY_KPRIGHTPAREN", Self::KEY_KPRIGHTPAREN),
        ("KEY_NEW", Self::KEY_NEW),
        ("KEY_REDO", Self::KEY_REDO),
        ("KEY_F13", Self::KEY_F13),
        ("KEY_F14", Self::KEY_F14),
        ("KEY_F15", Self::KEY_F15),
        ("KEY_F16", Self::KEY_F16),
        ("KEY_F17", Self::KEY_F17),
        ("KEY_F18", Self::KEY_F18),
        ("KEY_F19", Self::KEY_F19),
        ("KEY_F20", Self::KEY_F20),
        ("KEY_F21", Self::KEY_F21),
        ("KEY_F22", Self::KEY_F22),
        ("KEY_F23", Self::KEY_F23),
        ("KEY_F24", Self::KEY_F24),
        ("KEY_PLAYCD", Self::KEY_PLAYCD),
        ("KEY_PAUSECD", Self::KEY_PAUSECD),
        ("KEY_PROG3", Self::KEY_PROG3),
        ("KEY_PROG4", Self::KEY_PROG4),
        ("KEY_DASHBOARD", Self::KEY_DASHBOARD),
        ("KEY_SUSPEND", Self::KEY_SUSPEND),
        ("KEY_CLOSE", Self::KEY_CLOSE),
        ("KEY_PLAY", Self::KEY_PLAY),
        ("KEY_FASTFORWARD", Self::KEY_FASTFORWARD),
        ("KEY_BASSBOOST", Self::KEY_BASSBOOST),
        ("KEY_PRINT", Self::KEY_PRINT),
        ("KEY_HP", Self::KEY_HP),
        ("KEY_CAMERA", Self::KEY_CAMERA),
        ("KEY_SOUND", Self::KEY_SOUND),
        ("KEY_QUESTION", Self::KEY_QUESTION),
        ("KEY_EMAIL", Self::KEY_EMAIL),
        ("KEY_CHAT", Self::KEY_CHAT),
        ("KEY_SEARCH", Self::KEY_SEARCH),
        ("KEY_CONNECT", Self::KEY_CONNECT),
        ("KEY_FINANCE", Self::KEY_FINANCE),
        ("KEY_SPORT", Self::KEY_SPORT),
        ("KEY_SHOP", Self::KEY_SHOP),
        ("KEY_ALTERASE", Self::KEY_ALTERASE),
        ("KEY_CANCEL", Self::KEY_CANCEL),
        ("KEY_BRIGHTNESSDOWN", Self::KEY_BRIGHTNESSDOWN),
        ("KEY_BRIGHTNESSUP", Self::KEY_BRIGHTNESSUP),
        ("KEY_MEDIA", Self::KEY_MEDIA),
        ("KEY_SWITCHVIDEOMODE", Self::KEY_SWITCHVIDEOMODE),
        ("KEY_KBDILLUMTOGGLE", Self::KEY_KBDILLUMTOGGLE),
        ("KEY_KBDILLUMDOWN", Self::KEY_KBDILLUMDOWN),
        ("KEY_KBDILLUMUP", Self::KEY_KBDILLUMUP),
        ("KEY_SEND", Self::KEY_SEND),
        ("KEY_REPLY", Self::KEY_REPLY),
        ("KEY_FORWARDMAIL", Self::KEY_FORWARDMAIL),
        ("KEY_SAVE", Self::KEY_SAVE),
        ("KEY_DOCUMENTS", Self::KEY_DOCUMENTS),
        ("KEY_BATTERY", Self::KEY_BATTERY),
        ("KEY_BLUETOOTH", Self::KEY_BLUETOOTH),
        ("KEY_WLAN", Self::KEY_WLAN),
        ("KEY_UWB", Self::KEY_UWB),
        ("KEY_UNKNOWN", Self::KEY_UNKNOWN),
        ("KEY_VIDEO_NEXT", Self::KEY_VIDEO_NEXT),
        ("KEY_VIDEO_PREV", Self::KEY_VIDEO_PREV),
        ("KEY_BRIGHTNESS_CYCLE", Self::KEY_BRIGHTNESS_CYCLE),
        ("KEY_BRIGHTNESS_AUTO", Self::KEY_BRIGHTNESS_AUTO),
        ("KEY_DISPLAY_OFF", Self::KEY_DISPLAY_OFF),
        ("KEY_WWAN", Self::KEY_WWAN),
        ("KEY_RFKILL", Self::KEY_RFKILL),
        ("KEY_MICMUTE", Self::KEY_MICMUTE),
        ("BTN_0", Self::BTN_0),
        ("BTN_1", Self::BTN_1),
        ("BTN_2", Self::BTN_2),
        ("BTN_3", Self::BTN_3),
        ("BTN_4", Self::BTN_4),
        ("BTN_5", Self::BTN_5),
        ("BTN_6", Self::BTN_6),
        ("BTN_7", Self::BTN_7),
        ("BTN_8", Self::BTN_8),
        ("BTN_9", Self::BTN_9),
        ("BTN_LEFT", Self::BTN_LEFT),
        ("BTN_RIGHT", Self::BTN_RIGHT),
        ("BTN_MIDDLE", Self::BTN_MIDDLE),
        ("BTN_SIDE", Self::BTN_SIDE),
        ("BTN_EXTRA", Self::BTN_EXTRA),
        ("BTN_FORWARD", Self::BTN_FORWARD),
        ("BTN_BACK", Self::BTN_BACK),
        ("BTN_TASK", Self::BTN_TASK),
        ("BTN_TRIGGER", Self::BTN_TRIGGER),
        ("BTN_THUMB", Self::BTN_THUMB),
        ("BTN_THUMB2", Self::BTN_THUMB2),
        ("BTN_TOP", Self::BTN_TOP),
        ("BTN_TOP2", Self::BTN_TOP2),
        ("BTN_PINKIE", Self::BTN_PINKIE),
        ("BTN_BASE", Self::BTN_BASE),
        ("BTN_BASE2", Self::BTN_BASE2),
        ("BTN_BASE3", Self::BTN_BASE3),
        ("BTN_BASE4", Self::BTN_BASE4),
        ("BTN_BASE5", Self::BTN_BASE5),
        ("BTN_BASE6", Self::BTN_BASE6),
        ("BTN_DEAD", Self::BTN_DEAD),
        ("BTN_SOUTH", Self::BTN_SOUTH),
        ("BTN_EAST", Self::BTN_EAST),
        ("BTN_C", Self::BTN_C),
        ("BTN_NORTH", Self::BTN_NORTH),
        ("BTN_WEST", Self::BTN_WEST),
        ("BTN_Z", Self::BTN_Z),
        ("BTN_TL", Self::BTN_TL),
        ("BTN_TR", Self::BTN_TR),
        ("BTN_TL2", Self::BTN_TL2),
        ("BTN_TR2", Self::BTN_TR2),
        ("BTN_SELECT", Self::BTN_SELECT),
        ("BTN_START", Self::BTN_START),
        ("BTN_MODE", Self::BTN_MODE),
        ("BTN_THUMBL", Self::BTN_THUMBL),
        ("BTN_THUMBR", Self::BTN_THUMBR),
        ("BTN_TOOL_PEN", Self::BTN_TOOL_PEN),
        ("BTN_TOOL_RUBBER", Self::BTN_TOOL_RUBBER),
        ("BTN_TOOL_BRUSH", Self::BTN_TOOL_BRUSH),
        ("BTN_TOOL_PENCIL", Self::BTN_TOOL_PENCIL),
        ("BTN_TOOL_AIRBRUSH", Self::BTN_TOOL_AIRBRUSH),
        ("BTN_TOOL_FINGER", Self::BTN_TOOL_FINGER),
        ("BTN_TOOL_MOUSE", Self::BTN_TOOL_MOUSE),
        ("BTN_TOOL_LENS", Self::BTN_TOOL_LENS),
        ("BTN_TOOL_QUINTTAP", Self::BTN_TOOL_QUINTTAP),
        ("BTN_TOUCH", Self::BTN_TOUCH),
        ("BTN_STYLUS", Self::BTN_STYLUS),
        ("BTN_STYLUS2", Self::BTN_STYLUS2),
        ("BTN_TOOL_DOUBLETAP", Self::BTN_TOOL_DOUBLETAP),
        ("BTN_TOOL_TRIPLETAP", Self::BTN_TOOL_TRIPLETAP),
        ("BTN_TOOL_QUADTAP", Self::BTN_TOOL_QUADTAP),
        ("BTN_GEAR_DOWN", Self::BTN_GEAR_DOWN),
        ("BTN_GEAR_UP", Self::BTN_GEAR_UP),
        ("KEY_OK", Self::KEY_OK),
        ("KEY_SELECT", Self::KEY_SELECT),
        ("KEY_GOTO", Self::KEY_GOTO),
        ("KEY_CLEAR", Self::KEY_CLEAR),
        ("KEY_POWER2", Self::KEY_POWER2),
        ("KEY_OPTION", Self::KEY_OPTION),
        ("KEY_INFO", Self::KEY_INFO),
        ("KEY_TIME", Self::KEY_TIME),
        ("KEY_VENDOR", Self::KEY_VENDOR),
        ("KEY_ARCHIVE", Self::KEY_ARCHIVE),
        ("KEY_PROGRAM", Self::KEY_PROGRAM),
        ("KEY_CHANNEL", Self::KEY_CHANNEL),
        ("KEY_FAVORITES", Self::KEY_FAVORITES),
        ("KEY_EPG", Self::KEY_EPG),
        ("KEY_PVR", Self::KEY_PVR),
        ("KEY_MHP", Self::KEY_MHP),
        ("KEY_LANGUAGE", Self::KEY_LANGUAGE),
        ("KEY_TITLE", Self::KEY_TITLE),
        ("KEY_SUBTITLE", Self::KEY_SUBTITLE),
        ("KEY_ANGLE", Self::KEY_ANGLE),
        ("KEY_ZOOM", Self::KEY_ZOOM),
        ("KEY_FULL_SCREEN", Self::KEY_FULL_SCREEN),
        ("KEY_MODE", Self::KEY_MODE),
        ("KEY_KEYBOARD", Self::KEY_KEYBOARD),
        ("KEY_SCREEN", Self::KEY_SCREEN),
        ("KEY_PC", Self::KEY_PC),
        ("KEY_TV", Self::KEY_TV),
        ("KEY_TV2", Self::KEY_TV2),
        ("KEY_VCR", Self::KEY_VCR),
        ("KEY_VCR2", Self::KEY_VCR2),
        ("KEY_SAT", Self::KEY_SAT),
        ("KEY_SAT2", Self::KEY_SAT2),
        ("KEY_CD", Self::KEY_CD),
        ("KEY_TAPE", Self::KEY_TAPE),
        ("KEY_RADIO", Self::KEY_RADIO),
        ("KEY_TUNER", Self::KEY_TUNER),
        ("KEY_PLAYER", Self::KEY_PLAYER),
        ("KEY_TEXT", Self::KEY_TEXT),
        ("KEY_DVD", Self::KEY_DVD),
        ("KEY_AUX", Self::KEY_AUX),
        ("KEY_MP3", Self::KEY_MP3),
        ("KEY_AUDIO", Self::KEY_AUDIO),
        ("KEY_VIDEO", Self::KEY_VIDEO),
        ("KEY_DIRECTORY", Self::KEY_DIRECTORY),
        ("KEY_LIST", Self::KEY_LIST),
        ("KEY_MEMO", Self::KEY_MEMO),
        ("KEY_CALENDAR", Self::KEY_CALENDAR),
        ("KEY_RED", Self::KEY_RED),
        ("KEY_GREEN", Self::KEY_GREEN),
        ("KEY_YELLOW", Self::KEY_YELLOW),
        ("KEY_BLUE", Self::KEY_BLUE),
        ("KEY_CHANNELUP", Self::KEY_CHANNELUP),
        ("KEY_CHANNELDOWN", Self::KEY_CHANNELDOWN),
        ("KEY_FIRST", Self::KEY_FIRST),
        ("KEY_LAST", Self::KEY_LAST),
        ("KEY_AB", Self::KEY_AB),
        ("KEY_NEXT", Self::KEY_NEXT),
        ("KEY_RESTART", Self::KEY_RESTART),
        ("KEY_SLOW", Self::KEY_SLOW),
        ("KEY_SHUFFLE", Self::KEY_SHUFFLE),
        ("KEY_BREAK", Self::KEY_BREAK),
        ("KEY_PREVIOUS", Self::KEY_PREVIOUS),
        ("KEY_DIGITS", Self::KEY_DIGITS),
        ("KEY_TEEN", Self::KEY_TEEN),
        ("KEY_TWEN", Self::KEY_TWEN),
        ("KEY_VIDEOPHONE", Self::KEY_VIDEOPHONE),
        ("KEY_GAMES", Self::KEY_GAMES),
        ("KEY_ZOOMIN", Self::KEY_ZOOMIN),
        ("KEY_ZOOMOUT", Self::KEY_ZOOMOUT),
        ("KEY_ZOOMRESET", Self::KEY_ZOOMRESET),
        ("KEY_WORDPROCESSOR", Self::KEY_WORDPROCESSOR),
        ("KEY_EDITOR", Self::KEY_EDITOR),
        ("KEY_SPREADSHEET", Self::KEY_SPREADSHEET),
        ("KEY_GRAPHICSEDITOR", Self::KEY_GRAPHICSEDITOR),
        ("KEY_PRESENTATION", Self::KEY_PRESENTATION),
        ("KEY_DATABASE", Self::KEY_DATABASE),
        ("KEY_NEWS", Self::KEY_NEWS),
        ("KEY_VOICEMAIL", Self::KEY_VOICEMAIL),
        ("KEY_ADDRESSBOOK", Self::KEY_ADDRESSBOOK),
        ("KEY_MESSENGER", Self::KEY_MESSENGER),
        ("KEY_DISPLAYTOGGLE", Self::KEY_DISPLAYTOGGLE),
        ("KEY_SPELLCHECK", Self::KEY_SPELLCHECK),
        ("KEY_LOGOFF", Self::KEY_LOGOFF),
        ("KEY_DOLLAR", Self::KEY_DOLLAR),
        ("KEY_EURO", Self::KEY_EURO),
        ("KEY_FRAMEBACK", Self::KEY_FRAMEBACK),
        ("KEY_FRAMEFORWARD", Self::KEY_FRAMEFORWARD),
        ("KEY_CONTEXT_MENU", Self::KEY_CONTEXT_MENU),
        ("KEY_MEDIA_REPEAT", Self::KEY_MEDIA_REPEAT),
        ("KEY_10CHANNELSUP", Self::KEY_10CHANNELSUP),
        ("KEY_10CHANNELSDOWN", Self::KEY_10CHANNELSDOWN),
        ("KEY_IMAGES", Self::KEY_IMAGES),
        ("KEY_PICKUP_PHONE", Self::KEY_PICKUP_PHONE),
        ("KEY_HANGUP_PHONE", Self::KEY_HANGUP_PHONE),
        ("KEY_DEL_EOL", Self::KEY_DEL_EOL),
        ("KEY_DEL_EOS", Self::KEY_DEL_EOS),
        ("KEY_INS_LINE", Self::KEY_INS_LINE),
        ("KEY_DEL_LINE", Self::KEY_DEL_LINE),
        ("KEY_FN", Self::KEY_FN),
        ("KEY_FN_ESC", Self::KEY_FN_ESC),
        ("KEY_FN_F1", Self::KEY_FN_F1),
        ("KEY_FN_F2", Self::KEY_FN_F2),
        ("KEY_FN_F3", Self::KEY_FN_F3),
        ("KEY_FN_F4", Self::KEY_FN_F4),
        ("KEY_FN_F5", Self::KEY_FN_F5),
        ("KEY_FN_F6", Self::KEY_FN_F6),
        ("KEY_FN_F7", Self::KEY_FN_F7),
        ("KEY_FN_F8", Self::KEY_FN_F8),
        ("KEY_FN_F9", Self::KEY_FN_F9),
        ("KEY_FN_F10", Self::KEY_FN_F10),
        ("KEY_FN_F11", Self::KEY_FN_F11),
        ("KEY_FN_F12", Self::KEY_FN_F12),
        ("KEY_FN_1", Self::KEY_FN_1),
        ("KEY_FN_2", Self::KEY_FN_2),
        ("KEY_FN_D", Self::KEY_FN_D),
        ("KEY_FN_E", Self::KEY_FN_E),
        ("KEY_FN_F", Self::KEY_FN_F),
        ("KEY_FN_S", Self::KEY_FN_S),
        ("KEY_FN_B", Self::KEY_FN_B),
        ("KEY_BRL_DOT1", Self::KEY_BRL_DOT1),
        ("KEY_BRL_DOT2", Self::KEY_BRL_DOT2),
        ("KEY_BRL_DOT3", Self::KEY_BRL_DOT3),
        ("KEY_BRL_DOT4", Self::KEY_BRL_DOT4),
        ("KEY_BRL_DOT5", Self::KEY_BRL_DOT5),
        ("KEY_BRL_DOT6", Self::KEY_BRL_DOT6),
        ("KEY_BRL_DOT7", Self::KEY_BRL_DOT7),
        ("KEY_BRL_DOT8", Self::KEY_BRL_DOT8),
        ("KEY_BRL_DOT9", Self::KEY_BRL_DOT9),
        ("KEY_BRL_DOT10", Self::KEY_BRL_DOT10),
        ("KEY_NUMERIC_0", Self::KEY_NUMERIC_0),
        ("KEY_NUMERIC_1", Self::KEY_NUMERIC_1),
        ("KEY_NUMERIC_2", Self::KEY_NUMERIC_2),
        ("KEY_NUMERIC_3", Self::KEY_NUMERIC_3),
        ("KEY_NUMERIC_4", Self::KEY_NUMERIC_4),
        ("KEY_NUMERIC_5", Self::KEY_NUMERIC_5),
        ("KEY_NUMERIC_6", Self::KEY_NUMERIC_6),
        ("KEY_NUMERIC_7", Self::KEY_NUMERIC_7),
        ("KEY_NUMERIC_8", Self::KEY_NUMERIC_8),
        ("KEY_NUMERIC_9", Self::KEY_NUMERIC_9),
        ("KEY_NUMERIC_STAR", Self::KEY_NUMERIC_STAR),
        ("KEY_NUMERIC_POUND", Self::KEY_NUMERIC_POUND),
        ("KEY_NUMERIC_A", Self::KEY_NUMERIC_A),
        ("KEY_NUMERIC_B", Self::KEY_NUMERIC_B),
        ("KEY_NUMERIC_C", Self::KEY_NUMERIC_C),
        ("KEY_NUMERIC_D", Self::KEY_NUMERIC_D),
        ("KEY_CAMERA_FOCUS", Self::KEY_CAMERA_FOCUS),
        ("KEY_WPS_BUTTON", Self::KEY_WPS_BUTTON),
        ("KEY_TOUCHPAD_TOGGLE", Self::KEY_TOUCHPAD_TOGGLE),
        ("KEY_TOUCHPAD_ON", Self::KEY_TOUCHPAD_ON),
        ("KEY_TOUCHPAD_OFF", Self::KEY_TOUCHPAD_OFF),
        ("KEY_CAMERA_ZOOMIN", Self::KEY_CAMERA_ZOOMIN),
        ("KEY_CAMERA_ZOOMOUT", Self::KEY_CAMERA_ZOOMOUT),
        ("KEY_CAMERA_UP", Self::KEY_CAMERA_UP),
        ("KEY_CAMERA_DOWN", Self::KEY_CAMERA_DOWN),
        ("KEY_CAMERA_LEFT", Self::KEY_CAMERA_LEFT),
        ("KEY_CAMERA_RIGHT", Self::KEY_CAMERA_RIGHT),
        ("KEY_ATTENDANT_ON", Self::KEY_ATTENDANT_ON),
        ("KEY_ATTENDANT_OFF", Self::KEY_ATTENDANT_OFF),
        ("KEY_ATTENDANT_TOGGLE", Self::KEY_ATTENDANT_TOGGLE),
        ("KEY_LIGHTS_TOGGLE", Self::KEY_LIGHTS_TOGGLE),
        ("BTN_DPAD_UP", Self::BTN_DPAD_UP),
        ("BTN_DPAD_DOWN", Self::BTN_DPAD_DOWN),
        ("BTN_DPAD_LEFT", Self::BTN_DPAD_LEFT),
        ("BTN_DPAD_RIGHT", Self::BTN_DPAD_RIGHT),
        ("KEY_ALS_TOGGLE", Self::KEY_ALS_TOGGLE),
        ("KEY_BUTTONCONFIG", Self::KEY_BUTTONCONFIG),
        ("KEY_TASKMANAGER", Self::KEY_TASKMANAGER),
        ("KEY_JOURNAL", Self::KEY_JOURNAL),
        ("KEY_CONTROLPANEL", Self::KEY_CONTROLPANEL),
        ("KEY_APPSELECT", Self::KEY_APPSELECT),
        ("KEY_SCREENSAVER", Self::KEY_SCREENSAVER),
        ("KEY_VOICECOMMAND", Self::KEY_VOICECOMMAND),
        ("KEY_ASSISTANT", Self::KEY_ASSISTANT),
        ("KEY_KBD_LAYOUT_NEXT", Self::KEY_KBD_LAYOUT_NEXT),
        ("KEY_BRIGHTNESS_MIN", Self::KEY_BRIGHTNESS_MIN),
        ("KEY_BRIGHTNESS_MAX", Self::KEY_BRIGHTNESS_MAX),
        ("KEY_KBDINPUTASSIST_PREV", Self::KEY_KBDINPUTASSIST_PREV),
        ("KEY_KBDINPUTASSIST_NEXT", Self::KEY_KBDINPUTASSIST_NEXT),
        ("KEY_KBDINPUTASSIST_PREVGROUP", Self::KEY_KBDINPUTASSIST_PREVGROUP),
        ("KEY_KBDINPUTASSIST_NEXTGROUP", Self::KEY_KBDINPUTASSIST_NEXTGROUP),
        ("KEY_KBDINPUTASSIST_ACCEPT", Self::KEY_KBDINPUTASSIST_ACCEPT),
        ("KEY_KBDINPUTASSIST_CANCEL", Self::KEY_KBDINPUTASSIST_CANCEL),
        ("KEY_RIGHT_UP", Self::KEY_RIGHT_UP),
        ("KEY_RIGHT_DOWN", Self::KEY_RIGHT_DOWN),
        ("KEY_LEFT_UP", Self::KEY_LEFT_UP),
        ("KEY_LEFT_DOWN", Self::KEY_LEFT_DOWN),
        ("KEY_ROOT_MENU", Self::KEY_ROOT_MENU),
        ("KEY_MEDIA_TOP_MENU", Self::KEY_MEDIA_TOP_MENU),
        ("KEY_NUMERIC_11", Self::KEY_NUMERIC_11),
        ("KEY_NUMERIC_12", Self::KEY_NUMERIC_12),
        ("KEY_AUDIO_DESC", Self::KEY_AUDIO_DESC),
        ("KEY_3D_MODE", Self::KEY_3D_MODE),
        ("KEY_NEXT_FAVORITE", Self::KEY_NEXT_FAVORITE),
        ("KEY_STOP_RECORD", Self::KEY_STOP_RECORD),
        ("KEY_PAUSE_RECORD", Self::KEY_PAUSE_RECORD),
        ("KEY_VOD", Self::KEY_VOD),
        ("KEY_UNMUTE", Self::KEY_UNMUTE),
        ("KEY_FASTREVERSE", Self::KEY_FASTREVERSE),
        ("KEY_SLOWREVERSE", Self::KEY_SLOWREVERSE),
        ("KEY_DATA", Self::KEY_DATA),
        ("KEY_ONSCREEN_KEYBOARD", Self::KEY_ONSCREEN_KEYBOARD),
        ("KEY_PRIVACY_SCREEN_TOGGLE", Self::KEY_PRIVACY_SCREEN_TOGGLE),
        ("KEY_SELECTIVE_SCREENSHOT", Self::KEY_SELECTIVE_SCREENSHOT),
        ("BTN_TRIGGER_HAPPY1", Self::BTN_TRIGGER_HAPPY1),
        ("BTN_TRIGGER_HAPPY2", Self::BTN_TRIGGER_HAPPY2),
        ("BTN_TRIGGER_HAPPY3", Self::BTN_TRIGGER_HAPPY3),
        ("BTN_TRIGGER_HAPPY4", Self::BTN_TRIGGER_HAPPY4),
        ("BTN_TRIGGER_HAPPY5", Self::BTN_TRIGGER_HAPPY5),
        ("BTN_TRIGGER_HAPPY6", Self::BTN_TRIGGER_HAPPY6),
        ("BTN_TRIGGER_HAPPY7", Self::BTN_TRIGGER_HAPPY7),
        ("BTN_TRIGGER_HAPPY8", Self::BTN_TRIGGER_HAPPY8),
        ("BTN_TRIGGER_HAPPY9", Self::BTN_TRIGGER_HAPPY9),
        ("BTN_TRIGGER_HAPPY10", Self::BTN_TRIGGER_HAPPY10),
        ("BTN_TRIGGER_HAPPY11", Self::BTN_TRIGGER_HAPPY11),
        ("BTN_TRIGGER_HAPPY12", Self::BTN_TRIGGER_HAPPY12),
        ("BTN_TRIGGER_HAPPY13", Self::BTN_TRIGGER_HAPPY13),
        ("BTN_TRIGGER_HAPPY14", Self::BTN_TRIGGER_HAPPY14),
        ("BTN_TRIGGER_HAPPY15", Self::BTN_TRIGGER_HAPPY15),
        ("BTN_TRIGGER_HAPPY16", Self::BTN_TRIGGER_HAPPY16),
        ("BTN_TRIGGER_HAPPY17", Self::BTN_TRIGGER_HAPPY17),
        ("BTN_TRIGGER_HAPPY18", Self::BTN_TRIGGER_HAPPY18),
        ("BTN_TRIGGER_HAPPY19", Self::BTN_TRIGGER_HAPPY19),
        ("BTN_TRIGGER_HAPPY20", Self::BTN_TRIGGER_HAPPY20),
        ("BTN_TRIGGER_HAPPY21", Self::BTN_TRIGGER_HAPPY21),
        ("BTN_TRIGGER_HAPPY22", Self::BTN_TRIGGER_HAPPY22),
        ("BTN_TRIGGER_HAPPY23", Self::BTN_TRIGGER_HAPPY23),
        ("BTN_TRIGGER_HAPPY24", Self::BTN_TRIGGER_HAPPY24),
        ("BTN_TRIGGER_HAPPY25", Self::BTN_TRIGGER_HAPPY25),
        ("BTN_TRIGGER_HAPPY26", Self::BTN_TRIGGER_HAPPY26),
        ("BTN_TRIGGER_HAPPY27", Self::BTN_TRIGGER_HAPPY27),
        ("BTN_TRIGGER_HAPPY28", Self::BTN_TRIGGER_HAPPY28),
        ("BTN_TRIGGER_HAPPY29", Self::BTN_TRIGGER_HAPPY29),
        ("BTN_TRIGGER_HAPPY30", Self::BTN_TRIGGER_HAPPY30),
        ("BTN_TRIGGER_HAPPY31", Self::BTN_TRIGGER_HAPPY31),
        ("BTN_TRIGGER_HAPPY32", Self::BTN_TRIGGER_HAPPY32),
        ("BTN_TRIGGER_HAPPY33", Self::BTN_TRIGGER_HAPPY33),
        ("BTN_TRIGGER_HAPPY34", Self::BTN_TRIGGER_HAPPY34),
        ("BTN_TRIGGER_HAPPY35", Self::BTN_TRIGGER_HAPPY35),
        ("BTN_TRIGGER_HAPPY36", Self::BTN_TRIGGER_HAPPY36),
        ("BTN_TRIGGER_HAPPY37", Self::BTN_TRIGGER_HAPPY37),
        ("BTN_TRIGGER_HAPPY38", Self::BTN_TRIGGER_HAPPY38),
        ("BTN_TRIGGER_HAPPY39", Self::BTN_TRIGGER_HAPPY39),
        ("BTN_TRIGGER_HAPPY40", Self::BTN_TRIGGER_HAPPY40),
    ];
}
