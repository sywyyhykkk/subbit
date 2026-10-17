const levelDefaults = [
  ["0", "订阅萌新", "Subscription Newcomer"],
  ["100", "数字玩家", "Digital Explorer"],
  ["300", "效率达人", "Productivity Pro"],
  ["600", "云端收藏家", "Cloud Collector"],
  ["1000", "数字土豪", "Digital Tycoon"],
] as const;

export function defaultLevels(en = false) {
  return levelDefaults.map(([above, zh, english]) => ({ above, name: en ? english : zh }));
}

export function localizeLevelName(name: string, en: boolean) {
  const level = levelDefaults.find(([, zh, english]) => name === zh || name === english);
  return level ? level[en ? 2 : 1] : name;
}
