// First languages for Karaoke Studio: proper language names (never a
// nationality standing in for a language), with the regional accents where
// they're commonly told apart. Searchable by English or native name.

export interface Language {
  /** BCP 47 tag: "es-MX", "hi", "zh-CN". */
  code: string;
  /** "Spanish (Mexico)". */
  name: string;
  /** "Español (México)". */
  native: string;
}

const L = (code: string, name: string, native: string): Language => ({ code, name, native });

export const LANGUAGES: Language[] = [
  L('es-MX', 'Spanish (Mexico)', 'Español (México)'),
  L('es-ES', 'Spanish (Spain)', 'Español (España)'),
  L('es-AR', 'Spanish (Argentina)', 'Español (Argentina)'),
  L('es-CO', 'Spanish (Colombia)', 'Español (Colombia)'),
  L('es-CL', 'Spanish (Chile)', 'Español (Chile)'),
  L('es-PE', 'Spanish (Peru)', 'Español (Perú)'),
  L('es-VE', 'Spanish (Venezuela)', 'Español (Venezuela)'),
  L('es-CU', 'Spanish (Cuba)', 'Español (Cuba)'),
  L('es-US', 'Spanish (United States)', 'Español (Estados Unidos)'),
  L('en-US', 'English (United States)', 'English (US)'),
  L('en-GB', 'English (United Kingdom)', 'English (UK)'),
  L('en-AU', 'English (Australia)', 'English (Australia)'),
  L('en-CA', 'English (Canada)', 'English (Canada)'),
  L('en-IE', 'English (Ireland)', 'English (Ireland)'),
  L('en-IN', 'English (India)', 'English (India)'),
  L('en-NG', 'English (Nigeria)', 'English (Nigeria)'),
  L('en-ZA', 'English (South Africa)', 'English (South Africa)'),
  L('pt-BR', 'Portuguese (Brazil)', 'Português (Brasil)'),
  L('pt-PT', 'Portuguese (Portugal)', 'Português (Portugal)'),
  L('fr-FR', 'French (France)', 'Français (France)'),
  L('fr-CA', 'French (Canada)', 'Français (Canada)'),
  L('fr-BE', 'French (Belgium)', 'Français (Belgique)'),
  L('fr-CH', 'French (Switzerland)', 'Français (Suisse)'),
  L('de-DE', 'German (Germany)', 'Deutsch (Deutschland)'),
  L('de-AT', 'German (Austria)', 'Deutsch (Österreich)'),
  L('de-CH', 'German (Switzerland)', 'Deutsch (Schweiz)'),
  L('it-IT', 'Italian', 'Italiano'),
  L('ca', 'Catalan', 'Català'),
  L('gl', 'Galician', 'Galego'),
  L('eu', 'Basque', 'Euskara'),
  L('nl-NL', 'Dutch (Netherlands)', 'Nederlands (Nederland)'),
  L('nl-BE', 'Dutch (Belgium, Flemish)', 'Nederlands (België)'),
  L('ru', 'Russian', 'Русский'),
  L('uk', 'Ukrainian', 'Українська'),
  L('be', 'Belarusian', 'Беларуская'),
  L('pl', 'Polish', 'Polski'),
  L('cs', 'Czech', 'Čeština'),
  L('sk', 'Slovak', 'Slovenčina'),
  L('sl', 'Slovenian', 'Slovenščina'),
  L('hr', 'Croatian', 'Hrvatski'),
  L('sr', 'Serbian', 'Српски / Srpski'),
  L('bs', 'Bosnian', 'Bosanski'),
  L('bg', 'Bulgarian', 'Български'),
  L('mk', 'Macedonian', 'Македонски'),
  L('ro', 'Romanian', 'Română'),
  L('hu', 'Hungarian', 'Magyar'),
  L('el', 'Greek', 'Ελληνικά'),
  L('sq', 'Albanian', 'Shqip'),
  L('lt', 'Lithuanian', 'Lietuvių'),
  L('lv', 'Latvian', 'Latviešu'),
  L('et', 'Estonian', 'Eesti'),
  L('fi', 'Finnish', 'Suomi'),
  L('sv', 'Swedish', 'Svenska'),
  L('nb', 'Norwegian (Bokmål)', 'Norsk bokmål'),
  L('nn', 'Norwegian (Nynorsk)', 'Norsk nynorsk'),
  L('da', 'Danish', 'Dansk'),
  L('is', 'Icelandic', 'Íslenska'),
  L('ga', 'Irish', 'Gaeilge'),
  L('cy', 'Welsh', 'Cymraeg'),
  L('tr', 'Turkish', 'Türkçe'),
  L('az', 'Azerbaijani', 'Azərbaycan'),
  L('kk', 'Kazakh', 'Қазақ'),
  L('uz', 'Uzbek', 'Oʻzbek'),
  L('ka', 'Georgian', 'ქართული'),
  L('hy', 'Armenian', 'Հայերեն'),
  L('ar-EG', 'Arabic (Egypt)', 'العربية (مصر)'),
  L('ar-SA', 'Arabic (Saudi Arabia)', 'العربية (السعودية)'),
  L('ar-MA', 'Arabic (Morocco)', 'العربية (المغرب)'),
  L('ar-LB', 'Arabic (Levant)', 'العربية (الشام)'),
  L('he', 'Hebrew', 'עברית'),
  L('fa', 'Persian (Farsi)', 'فارسی'),
  L('ku', 'Kurdish', 'Kurdî'),
  L('ps', 'Pashto', 'پښتو'),
  L('ur', 'Urdu', 'اردو'),
  L('hi-IN', 'Hindi (India)', 'हिन्दी'),
  L('bn-BD', 'Bengali (Bangladesh)', 'বাংলা (বাংলাদেশ)'),
  L('bn-IN', 'Bengali (India)', 'বাংলা (ভারত)'),
  L('pa', 'Punjabi', 'ਪੰਜਾਬੀ'),
  L('gu', 'Gujarati', 'ગુજરાતી'),
  L('mr', 'Marathi', 'मराठी'),
  L('ta', 'Tamil', 'தமிழ்'),
  L('te', 'Telugu', 'తెలుగు'),
  L('kn', 'Kannada', 'ಕನ್ನಡ'),
  L('ml', 'Malayalam', 'മലയാളം'),
  L('ne', 'Nepali', 'नेपाली'),
  L('si', 'Sinhala', 'සිංහල'),
  L('zh-CN', 'Mandarin Chinese (Mainland)', '普通话（中国大陆）'),
  L('zh-TW', 'Mandarin Chinese (Taiwan)', '國語（台灣）'),
  L('yue', 'Cantonese', '粵語'),
  L('ja', 'Japanese', '日本語'),
  L('ko', 'Korean', '한국어'),
  L('vi', 'Vietnamese', 'Tiếng Việt'),
  L('th', 'Thai', 'ไทย'),
  L('lo', 'Lao', 'ລາວ'),
  L('km', 'Khmer', 'ខ្មែរ'),
  L('my', 'Burmese', 'မြန်မာ'),
  L('id', 'Indonesian', 'Bahasa Indonesia'),
  L('ms', 'Malay', 'Bahasa Melayu'),
  L('fil', 'Filipino (Tagalog)', 'Filipino'),
  L('mn', 'Mongolian', 'Монгол'),
  L('sw', 'Swahili', 'Kiswahili'),
  L('am', 'Amharic', 'አማርኛ'),
  L('yo', 'Yoruba', 'Yorùbá'),
  L('ig', 'Igbo', 'Igbo'),
  L('ha', 'Hausa', 'Hausa'),
  L('zu', 'Zulu', 'isiZulu'),
  L('xh', 'Xhosa', 'isiXhosa'),
  L('af', 'Afrikaans', 'Afrikaans'),
  L('so', 'Somali', 'Soomaali'),
  L('ht', 'Haitian Creole', 'Kreyòl ayisyen'),
  L('qu', 'Quechua', 'Runasimi'),
  L('gn', 'Guarani', 'Avañeʼẽ'),
  L('mi', 'Māori', 'Te reo Māori'),
  L('haw', 'Hawaiian', 'ʻŌlelo Hawaiʻi'),
  L('eo', 'Esperanto', 'Esperanto'),
];

export function languageOf(code: string | undefined): Language | undefined {
  return LANGUAGES.find((l) => l.code === code);
}

const fold = (s: string) => s.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase();

/** Languages whose English or native name (or code) contains every word typed. */
export function searchLanguages(q: string): Language[] {
  const words = fold(q).split(/\s+/).filter(Boolean);
  if (!words.length) return LANGUAGES;
  return LANGUAGES.filter((l) => {
    const hay = fold(`${l.name} ${l.native} ${l.code}`);
    return words.every((w) => hay.includes(w));
  });
}

/** The language a browser or Windows is set to, as a starting suggestion. */
export function suggestedLanguage(): Language | undefined {
  const tags = typeof navigator !== 'undefined' ? [...(navigator.languages ?? []), navigator.language] : [];
  for (const t of tags) {
    const exact = LANGUAGES.find((l) => l.code.toLowerCase() === t.toLowerCase());
    if (exact) return exact;
  }
  for (const t of tags) {
    const base = t.split('-')[0].toLowerCase();
    const any = LANGUAGES.find((l) => l.code.split('-')[0] === base);
    if (any) return any;
  }
  return undefined;
}
