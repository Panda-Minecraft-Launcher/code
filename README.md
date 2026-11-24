# Panda Minecraft Launcher

Panda Minecraft Launcher, Windows ve Linux için ücretsiz ve açık kaynaklı bir Minecraft başlatıcısıdır. Modlu oyun profilleri oluşturmanızı, mod ve modpack kurmanızı, Minecraft sürümleri ile Java kurulumlarını yönetmenizi ve oyunu tek arayüzden başlatmanızı sağlar. Masaüstü uygulaması Tauri (Rust), arayüz Vue 3 ile yazılmıştır.

Bu depo Panda Minecraft Launcher'ın kaynak kod deposudur; web sitesi ayrı bir depoda geliştirilir.

## Özellikler

- Birden fazla Minecraft profili (instance) oluşturma ve yönetme, profilleri paylaşma
- Mod ve modpack kurma
- Minecraft sürümleri, Java kurulumları ve oyun ayarlarını yönetme
- Oyunu masaüstü uygulamasından doğrudan başlatma
- Windows ve Linux için yerel uygulama

## İndirme

Hazır derlenmiş Windows ve Linux sürümleri [GitHub Releases](https://github.com/Panda-Minecraft-Launcher/code/releases) sayfasında.

## Depo yapısı

| Dizin                 | İçerik                                                            |
| --------------------- | ----------------------------------------------------------------- |
| `apps/app`            | Tauri ile yazılmış masaüstü başlatıcı (Rust)                      |
| `apps/app-frontend`   | Vue 3 arayüz                                                      |
| `apps/labrinth`       | Backend API servisi (Rust)                                        |
| `packages/app-lib`    | Başlatıcının çekirdeği: profiller, kurulum ve oyun başlatma (Rust) |

Diğer `apps/` ve `packages/` dizinleri yardımcı araçları ve paylaşılan kütüphaneleri içerir. Web sitesi bu depoda değil, ayrı bir depoda geliştirilir.

## Yerelde geliştirme

Gereksinimler:

- Node.js 24.15.0 veya üzeri
- pnpm 12.11.2 (`packageManager` alanında tanımlı)
- Rust 1.95.0 veya üzeri (Cargo workspace, edition 2024)
- [Tauri'nin platform gereksinimleri](https://v2.tauri.app/start/prerequisites/)

```bash
pnpm install
pnpm --filter @modrinth/app run tauri dev     # geliştirme
pnpm --filter @modrinth/app run tauri build   # dağıtım derlemesi
```

Ayrıntılar için ilgili dizinlerdeki `AGENTS.md` dosyalarına bakın.

## Lisans

Projenin kendi kaynak kodu AGPL-3.0-only lisansındadır. Üçüncü taraf bileşenler kendi lisanslarını korur; ayrıntılar için [LICENSE](LICENSE), [COPYING.md](COPYING.md) ve ilgili dizinlerdeki `LICENSE` dosyalarına bakın.

## Topluluk

- [GitHub Issues](https://github.com/Panda-Minecraft-Launcher/code/issues)
- [Pull requests](https://github.com/Panda-Minecraft-Launcher/code/pulls)
- [Releases](https://github.com/Panda-Minecraft-Launcher/code/releases)

## English

Panda Minecraft Launcher is a free, open-source Minecraft launcher for Windows and Linux. It lets you create modded instances, install mods and modpacks, manage Minecraft versions and Java installations, and launch the game from a single interface.

This is the source repository for Panda Minecraft Launcher; the website lives in a separate repository. The main directories are `apps/app` (Tauri desktop app), `apps/app-frontend` (Vue 3 UI), `apps/labrinth` (backend API), and `packages/app-lib` (launcher core).

To build locally, install Node.js >= 24.15.0, pnpm 12.11.2, and Rust (edition 2024), then run `pnpm install` and `pnpm --filter @modrinth/app run tauri build`. Downloads are on [GitHub Releases](https://github.com/Panda-Minecraft-Launcher/code/releases). Licensing details are in [COPYING.md](COPYING.md).
