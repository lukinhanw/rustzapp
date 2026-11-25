<div align="center">

# RustZapp

**Navegador ultraleve em Rust exclusivo para múltiplos WhatsApp Web com sessões e cookies independentes.**

[![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)](https://www.rust-lang.org/)
[![tao + wry](https://img.shields.io/badge/UI-tao%20%2B%20wry-00a884)](https://github.com/tauri-apps/wry)
[![WebView2](https://img.shields.io/badge/Runtime-Windows%20WebView2-0078d4?logo=microsoftedge)](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)
[![Binary Size](https://img.shields.io/badge/Execut%C3%A1vel-~960%20KB-brightgreen)](#como-compilar-e-executar)
[![Multi-Conta](https://img.shields.io/badge/Sess%C3%B5es-Cookies%20Independentes-25d366)](#como-funciona-o-isolamento)

</div>

---

## Sobre o projeto

O **RustZapp** (`WhatsAppBrowser.exe`) é um navegador desktop minimalista feito em **Rust** que faz uma única coisa: **abrir diretamente o WhatsApp Web** com suporte a **várias abas simultâneas**, cada uma com **cookies, LocalStorage, IndexedDB e cache totalmente isolados**.

Sem Electron, sem Node.js e sem empacotar um Chromium inteiro — ele usa o **Microsoft Edge WebView2** nativo do Windows, gerando um executável único de menos de **1 MB** com inicialização instantânea.

---

## Principais recursos

- **Abertura direta e exclusiva no WhatsApp Web**: abre imediatamente em `https://web.whatsapp.com/`, sem sair do WhatsApp: links recebidos nas conversas abrem no **navegador padrão do Windows** (só `http`, `https` e `mailto`), e a janela nunca navega para outros sites nem abre pop-ups.
- **Múltiplas contas com cookies independentes**: cada aba recebe um perfil isolado do WebView2 (`wa_1`, `wa_2`, ...), permitindo conectar vários números de WhatsApp na mesma janela sem conflito de sessão.
- **Barra de título customizada (frameless)**: remove a barra padrão do Windows e integra na mesma faixa de `40px`:
  - Ícone do aplicativo + área arrastável (com duplo clique para maximizar/restaurar e redimensionamento pelas bordas).
  - Abas roláveis com indicador visual de aba ativa.
  - Botões de **Nova aba (`+`)**, **Recarregar (`↻`)**, **Minimizar**, **Maximizar/Restaurar** e **Fechar**.
- **Renomear abas**: clique duplo no nome da aba para personalizar (ex.: `Trabalho`, `Pessoal`, `Suporte`).
- **Notificações desktop nativas do Windows**:
  - Permissões de notificação, reprodução de áudio e downloads liberadas automaticamente no WhatsApp Web (sem bloquear o alerta por falta de prompt).
  - Balões **Toast nativos do Windows** com nome do contato, prévia da mensagem, conta/aba de origem e clique direto para abrir a conversa.
- **Contador de notificações em tempo real**:
  - Badge verde por aba na barra superior.
  - Soma total no título da janela.
  - **Badge vermelho numérico** sobre o ícone na barra de tarefas padrão do Windows (`ITaskbarList4::SetOverlayIcon`).
  - **Ícone dinâmico na bandeja do sistema** (`Shell_NotifyIconW`), exibindo o contador ao vivo inclusive na barra [**WinRust (`topbar`)**](https://github.com/Lemxy/WinRust).
- **Downloads como no navegador**: ao baixar foto ou documento abre a janela **"Salvar como"** do Windows (Downloads e nome original já preenchidos; cancelar não baixa). Ao terminar, uma notificação abre o arquivo com um clique — programas e scripts (`.exe`, `.msi`, `.bat`...) abrem só a pasta, nunca são executados.
- **Persistência automática**: suas abas abertas, nomes personalizados e logins (QR Code) permanecem salvos entre reinicializações.
- **Senha para exibir o WhatsApp (opcional)**: com senha definida, o app abre bloqueado e o cadeado da barra bloqueia na hora. Bloqueado, nomes de abas e contadores somem da barra e as notificações não mostram contato nem texto. Só um hash PBKDF2-SHA256 com sal fica em `tabs.json`. É uma trava de tela: os dados do WebView2 no disco não são criptografados.
- **Instância única amigável**: se o diretório de sessão já estiver em uso, restaura e foca a janela já aberta em vez de duplicar processos.

---

## Como usar

| Ação | Como fazer |
| :--- | :--- |
| **Abrir nova conta WhatsApp** | Clique no botão **`+`** na barra superior |
| **Alternar entre contas** | Clique na aba desejada |
| **Renomear aba** | **Clique duplo** no nome da aba e pressione `Enter` |
| **Fechar aba** | Clique no **`×`** da aba (ao fechar a última, uma nova aba limpa é criada) |
| **Recarregar aba ativa** | Clique no botão **`↻`** |
| **Mover / Maximizar janela** | Arraste pela área vazia da barra superior (duplo clique maximiza/restaura) |
| **Definir senha / bloquear** | Clique no **cadeado** da barra (sem senha: define; com senha: bloqueia na hora) |
| **Alterar ou remover a senha** | **Clique direito** no cadeado e confirme a senha atual |
| **Fixar contador na barra WinRust (`topbar`)** | Abra o chevron **`^`** da bandeja e clique com o **botão do meio** no ícone do RustZapp |

---

## Como funciona o isolamento

Todas as WebViews compartilham um único `WebContext` (otimizando memória e processos), mas cada aba é instanciada com um `profile_name` exclusivo no WebView2 (`wa_1`, `wa_2`, ...). Ao criar uma nova aba, o identificador incremental é salvo em `tabs.json`, garantindo que abas novas nunca herdem cookies de abas fechadas anteriormente.

```mermaid
flowchart LR
    W["Janela Principal (tao frameless)"] --> B["Barra Superior (perfil ui_bar)"]
    W --> T1["Aba 1: Trabalho (perfil wa_1)"]
    W --> T2["Aba 2: Pessoal (perfil wa_2)"]
    B -->|IPC| E["Event Loop + Tray / Overlay"]
    T1 -->|Título / Não lidas| E
    T2 -->|Título / Não lidas| E
```

---

## Estrutura do projeto

| Arquivo | Responsabilidade |
| :--- | :--- |
| [`src/main.rs`](src/main.rs) | Janela sem bordas (`tao`), criação das WebViews (`wry`), bloqueio de URLs/pop-ups, controle de abas, `TrayIcon` Win32 e instância única |
| [`src/config.rs`](src/config.rs) | Persistência em `tabs.json`, gerenciamento de perfis, parser de não lidas e allowlist de URLs |
| [`src/lock.rs`](src/lock.rs) | Senha para exibir o WhatsApp: hash PBKDF2-HMAC-SHA256 com sal e verificação em tempo constante |
| [`src/tab_bar.rs`](src/tab_bar.rs) | Barra superior HTML/CSS/JS autocontida com tema escuro do WhatsApp e comunicação IPC |
| [`src/badge.rs`](src/badge.rs) | Renderizador RGBA `32×32` em Rust puro para o ícone base da bandeja e badges numéricos (`1`–`99+`) |
| [`build.rs`](build.rs) | Embute `assets/icon.ico` e metadados no executável Windows via `winresource` |
| [`assets/icon.ico`](assets/icon.ico) | Ícone multi-resolução (`16`, `32`, `48`, `64` DIB + `256` PNG) |

---

## Como compilar e executar

### Pré-requisitos
- **Windows 10 ou Windows 11** (com **Microsoft Edge WebView2 Runtime**, já incluso nativamente na maioria dos sistemas).
- **Rust** (`stable-x86_64-pc-windows-msvc`, `rustc >= 1.85`) e **Visual Studio Build Tools** (MSVC + Windows SDK).

### Comandos

```powershell
# Clonar o repositório
git clone https://github.com/lukinhanw/rustzapp.git
cd rustzapp

# Executar testes unitários
cargo test

# Compilar versão otimizada para release
cargo build --release
```

O executável será gerado em:

```text
target\release\WhatsAppBrowser.exe
```

---

## Segurança e armazenamento local

- **Allowlist estrita de navegação**: apenas `https://web.whatsapp.com/`, `about:blank`, `data:` e `blob:https://web.whatsapp.com/` (para visualização/download de mídias do próprio WhatsApp) são permitidos.
- **Dados locais**: as sessões, cookies isolados e o arquivo `tabs.json` ficam armazenados exclusivamente na sua máquina em:

```text
%LOCALAPPDATA%\WhatsAppBrowser
```
