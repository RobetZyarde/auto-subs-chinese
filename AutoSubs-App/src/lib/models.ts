import { Model } from "@/types";

/**
 * Centralized model definitions for AutoSubs
 * These model definitions are used throughout the application
 * to ensure consistency in model information display
 */

/**
 * Predefined filter orders for models
 * Each array defines the optimal order for that filter type
 */
export const modelFilterOrders = {
  weight: [
    // 1GB RAM
    "moonshine-tiny", "moonshine-tiny-ar", "moonshine-tiny-zh", "moonshine-tiny-ja",
    "moonshine-tiny-ko", "moonshine-tiny-uk", "moonshine-tiny-vi",
    "moonshine-base",
    // 2GB RAM
    "moonshine-base-es", "parakeet",
    // 4GB+ RAM
    "qwen3-asr"
  ],
  accuracy: [
    "qwen3-asr", "parakeet", "moonshine-tiny-vi", "moonshine-tiny-ar", "moonshine-tiny-zh",
    "moonshine-tiny-ja", "moonshine-tiny-ko", "moonshine-base", "moonshine-base-es",
    "moonshine-tiny-uk", "moonshine-tiny"
  ],
  recommended: [
    "qwen3-asr", "parakeet", "moonshine-tiny-ar", "moonshine-tiny-zh", "moonshine-tiny-ja",
    "moonshine-tiny-ko", "moonshine-tiny-uk", "moonshine-tiny-vi", "moonshine-base",
    "moonshine-base-es", "moonshine-tiny"
  ]
};

export const models: Model[] = [
  {
    value: "qwen3-asr",
    label: "models.qwen3_asr.label",
    description: "models.qwen3_asr.description",
    size: "4GB+",
    ram: "8GB+ VRAM",
    image: "/phoenix.png",
    details: "models.qwen3_asr.details",
    badge: "models.qwen3_asr.badge",
    languageSupport: { kind: "multilingual" },
    accuracy: 4,
    weight: 1,
    isDownloaded: false,
  },
  {
    value: "parakeet",
    label: "models.parakeet.label",
    description: "models.parakeet.description",
    size: "700MB",
    ram: "2GB",
    image: "/parakeet.png",
    details: "models.parakeet.details",
    badge: "models.parakeet.badge",
    languageSupport: {
      kind: "restricted",
      languages: [
        "bg", "hr", "cs", "da", "nl", "en", "et", "fi", "fr", "de", "el", "hu", "it", "lv", "lt", "mt", "pl", "pt", "ro", "sk", "sl", "es", "sv", "ru", "uk"
      ],
    },
    accuracy: 3,
    weight: 3,
    isDownloaded: false,
  },
  {
    value: "moonshine-tiny",
    label: "models.moonshine_tiny.label",
    description: "models.moonshine_tiny.description",
    size: "60MB",
    ram: "1GB",
    image: "/bat.png",
    details: "models.moonshine_tiny.details",
    badge: "models.moonshine_tiny.badge",
    languageSupport: { kind: "single_language", language: "en" },
    accuracy: 1,
    weight: 4,
    isDownloaded: false,
  },
  {
    value: "moonshine-tiny-ar",
    label: "models.moonshine_tiny_ar.label",
    description: "models.moonshine_tiny_ar.description",
    size: "120MB",
    ram: "1GB",
    image: "/bat.png",
    details: "models.moonshine_tiny_ar.details",
    badge: "models.moonshine_tiny_ar.badge",
    languageSupport: { kind: "single_language", language: "ar" },
    accuracy: 3,
    weight: 4,
    isDownloaded: false,
  },
  {
    value: "moonshine-tiny-zh",
    label: "models.moonshine_tiny_zh.label",
    description: "models.moonshine_tiny_zh.description",
    size: "120MB",
    ram: "1GB",
    image: "/bat.png",
    details: "models.moonshine_tiny_zh.details",
    badge: "models.moonshine_tiny_zh.badge",
    languageSupport: { kind: "single_language", language: "zh" },
    accuracy: 3,
    weight: 4,
    isDownloaded: false,
  },
  {
    value: "moonshine-tiny-ja",
    label: "models.moonshine_tiny_ja.label",
    description: "models.moonshine_tiny_ja.description",
    size: "120MB",
    ram: "1GB",
    image: "/bat.png",
    details: "models.moonshine_tiny_ja.details",
    badge: "models.moonshine_tiny_ja.badge",
    languageSupport: { kind: "single_language", language: "ja" },
    accuracy: 3,
    weight: 4,
    isDownloaded: false,
  },
  {
    value: "moonshine-tiny-ko",
    label: "models.moonshine_tiny_ko.label",
    description: "models.moonshine_tiny_ko.description",
    size: "120MB",
    ram: "1GB",
    image: "/bat.png",
    details: "models.moonshine_tiny_ko.details",
    badge: "models.moonshine_tiny_ko.badge",
    languageSupport: { kind: "single_language", language: "ko" },
    accuracy: 3,
    weight: 4,
    isDownloaded: false,
  },
  {
    value: "moonshine-tiny-uk",
    label: "models.moonshine_tiny_uk.label",
    description: "models.moonshine_tiny_uk.description",
    size: "120MB",
    ram: "1GB",
    image: "/bat.png",
    details: "models.moonshine_tiny_uk.details",
    badge: "models.moonshine_tiny_uk.badge",
    languageSupport: { kind: "single_language", language: "uk" },
    accuracy: 2,
    weight: 4,
    isDownloaded: false,
  },
  {
    value: "moonshine-tiny-vi",
    label: "models.moonshine_tiny_vi.label",
    description: "models.moonshine_tiny_vi.description",
    size: "120MB",
    ram: "1GB",
    image: "/bat.png",
    details: "models.moonshine_tiny_vi.details",
    badge: "models.moonshine_tiny_vi.badge",
    languageSupport: { kind: "single_language", language: "vi" },
    accuracy: 3,
    weight: 4,
    isDownloaded: false,
  },
  {
    value: "moonshine-base",
    label: "models.moonshine_base.label",
    description: "models.moonshine_base.description",
    size: "200MB",
    ram: "1GB",
    image: "/owl.png",
    details: "models.moonshine_base.details",
    badge: "models.moonshine_base.badge",
    languageSupport: { kind: "single_language", language: "en" },
    accuracy: 2,
    weight: 4,
    isDownloaded: false,
  },
  {
    value: "moonshine-base-es",
    label: "models.moonshine_base_es.label",
    description: "models.moonshine_base_es.description",
    size: "350MB",
    ram: "2GB",
    image: "/owl.png",
    details: "models.moonshine_base_es.details",
    badge: "models.moonshine_base_es.badge",
    languageSupport: { kind: "single_language", language: "es" },
    accuracy: 2,
    weight: 3,
    isDownloaded: false,
  },
];

/**
 * Diarization model definition
 * This is handled separately from transcription models
 */
export const diarizeModel: Model = {
  value: "speaker-diarize",
  label: "models.diarize.label",
  description: "models.diarize.description",
  size: "40MB",
  ram: "",
  image: "/diarize.png",
  details: "models.diarize.details",
  badge: "models.diarize.badge",
  languageSupport: { kind: "multilingual" },
  accuracy: 3,
  weight: 3,
  isDownloaded: false, // Will be set to true when actually downloaded
};

/**
 * Check if a model supports a specific language
 */
export function modelSupportsLanguage(model: Model, language: string): boolean {
  if (language === "auto") return true

  switch (model.languageSupport.kind) {
    case "multilingual":
      return true
    case "single_language":
      return model.languageSupport.language === language
    case "restricted":
      return model.languageSupport.languages.includes(language)
    default:
      return true
  }
}

/**
 * Get the first recommended model that supports the given language
 */
export function getFirstRecommendedModelForLanguage(language: string): Model | null {
  // Use the general recommended order for all languages
  const order = modelFilterOrders.recommended
  
  for (const modelValue of order) {
    const model = models.find(m => m.value === modelValue)
    if (model && modelSupportsLanguage(model, language)) {
      return model
    }
  }
  
  return null
}
