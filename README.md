# sentinel

> Proteção de supply chain para projetos npm — instale com integridade ou não instale.

![build](https://img.shields.io/badge/build-passing-brightgreen)
![license](https://img.shields.io/badge/license-MIT-blue)
![platforms](https://img.shields.io/badge/platforms-linux%20%7C%20macos%20%7C%20windows-lightgrey)

Gerenciadores de pacote instalam rápido. O **sentinel** instala com segurança — validando integridade de tarball, lockfile e registry antes de qualquer coisa tocar seu projeto.

---

## Por que sentinel?

| | npm / yarn / pnpm | sentinel |
|---|---|---|
| Resolve e instala dependências | ✅ | ✅ |
| Valida integridade do tarball | ❌ | ✅ |
| Valida lockfile contra registry | ❌ | ✅ |
| Bloqueia pacote comprometido | ❌ | ✅ |
| Gate de segurança para CI | ❌ | ✅ |
| Saída para automação (json/junit/github) | ❌ | ✅ |

---

## Comece em 30 segundos

Escolha a abordagem que se encaixa no seu contexto:

### Opção A — sem instalar nada (npx)

Ideal para primeiro contato, ambientes efêmeros ou CI temporário:

```bash
# verificar projeto atual
npx -y -p sentinel-check ci

# instalar pacote com verificação
npx -y -p sentinel-check install <pacote>@<versao>
```

### Opção B — binário sentinel no PATH

Ideal para uso contínuo no time, substituindo `npm install` definitivamente:

```bash
# verificar projeto atual
sentinel ci

# instalar pacote com verificação
sentinel install <pacote>@<versao>
```

**Use a Opção A para adoção imediata. Migre para a Opção B quando o time estiver confortável.**

---

## Instalando o binário (Opção B)

### Linux e macOS

```bash
curl -fsSL https://github.com/SIG-sentinel/sentinel-npm/releases/latest/download/install.sh | sh
```

Isso baixa o binário correto para sua plataforma, verifica o checksum SHA-256 e instala em `~/.local/bin` (Linux) ou `/usr/local/bin` (macOS).

Confirme a instalação:

```bash
sentinel --version
```

### macOS via Homebrew _(em breve)_

```bash
brew install sig-sentinel/tap/sentinel
```

### Windows (PowerShell)

```powershell
irm https://github.com/SIG-sentinel/sentinel-npm/releases/latest/download/install.ps1 | iex
```

Instala em `%LOCALAPPDATA%\sentinel\bin` e adiciona ao PATH do usuário.

### Download manual

Acesse [github.com/SIG-sentinel/sentinel-npm/releases](https://github.com/SIG-sentinel/sentinel-npm/releases) e baixe o binário para sua plataforma:

| Plataforma | Arquivo |
|---|---|
| Linux x64 | `sentinel-linux-x64` |
| macOS x64 | `sentinel-darwin-x64` |
| macOS ARM (M1/M2/M3) | `sentinel-darwin-arm64` |
| Windows x64 | `sentinel-windows-x64.exe` |

Verifique o checksum com `checksums.txt` antes de executar.

---

## Padronize no package.json do seu projeto

Adicione ao `package.json` para que o time use sem depender de memória de comando:

**Com npx (sem instalação global):**

```json
{
  "scripts": {
    "sentinel:ci":      "npx -y -p sentinel-check ci",
    "sentinel:check":   "npx -y -p sentinel-check check",
    "sentinel:install": "npx -y -p sentinel-check install"
  }
}
```

**Com binário no PATH:**

```json
{
  "scripts": {
    "sentinel:ci":      "sentinel ci",
    "sentinel:check":   "sentinel check",
    "sentinel:install": "sentinel install"
  }
}
```

Uso:

```bash
npm run sentinel:ci
npm run sentinel:check
npm run sentinel:install -- express@4.21.2
```

---

## Como o sentinel protege seu projeto

Cada pacote passa por três camadas de verificação:

```
lockfile  ──►  registry  ──►  tarball
   │               │              │
   └── hash ok?    └── hash ok?   └── hash ok?
         │               │              │
       CLEAN           CLEAN          CLEAN
         │               │              │
        ❌ diverge     ❌ diverge     ❌ diverge
         │               │              │
    COMPROMISED    UNVERIFIABLE    COMPROMISED
```

- **CLEAN** — integridade confirmada, instalação permitida
- **UNVERIFIABLE** — não foi possível confirmar; instalação é bloqueada
- **COMPROMISED** — divergência detectada; instalação bloqueada sempre

---

## Comandos

```bash
sentinel check                     # auditoria sem instalar
sentinel install <pacote>@<versao> # instala pacote específico com verificação
sentinel ci                        # gate estrito para pipelines
sentinel report <pacote>           # relatório de um pacote
```

---

## Integração em CI/CD

### GitHub Actions

```yaml
- name: Verificar integridade de dependências
  run: npx -y -p sentinel-check ci
```

Ou, com binário instalado no runner:

```yaml
- name: Instalar sentinel
  run: curl -fsSL https://github.com/SIG-sentinel/sentinel-npm/releases/latest/download/install.sh | sh

- name: Verificar integridade de dependências
  run: sentinel ci
```

### Saída para automação

```bash
sentinel check --format json    # para parsing programático
sentinel check --format junit   # para relatórios de test suite
sentinel check --format github  # para anotações no PR (GitHub Actions)
```

> Sentinel instala apenas pacotes verificáveis.

---

## Variáveis de ambiente

| Variável | Descrição |
|---|---|
| `SENTINEL_BIN` | caminho para binário local existente |
| `SENTINEL_VERSION` | versão/tag específica do release |
| `SENTINEL_RELEASE_REPO` | override de repositório (`owner/repo`) |
| `SENTINEL_RELEASE_BASE_URL` | override da URL base de release |
| `SENTINEL_SKIP_DOWNLOAD=1` | desabilita download automático no wrapper npx |

---

## Arquitetura

```
sentinel
├── commands/       check · install · ci
├── verifier.rs     motor de verificação de integridade
├── npm.rs          integração com registry e lockfile
├── crypto.rs       hashing e validação SHA-256/SHA-512
├── cache.rs        cache local (SQLite)
├── policy/         decisão de bloqueio e modo strict
├── output.rs       renderização text · json · junit · github
├── types/          contratos por domínio
└── constants/      constantes por domínio
```

---

## Status

![build](https://img.shields.io/badge/build-passing-brightgreen)
![clippy](https://img.shields.io/badge/clippy-clean-brightgreen)
![tests](https://img.shields.io/badge/tests-passing-brightgreen)
