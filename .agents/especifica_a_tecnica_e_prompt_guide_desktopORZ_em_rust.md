# Guia de Especificação e Processo de Desenvolvimento: DesktopORZ em Rust

> **Público-alvo:** Modelos de Linguagem (LLMs) e Agentes Autônomos de Código.  
> **Objetivo:** Fornecer contexto técnico rigoroso, restrições e etapas sequenciais para implementação de um clone open source do DesktopOK usando Rust nativo no Windows.

---

## 1. Contexto do Sistema e Princípios Fundamentais

O aplicativo visa salvar e restaurar a disposição geométrica dos ícones do Desktop do Windows.

### A Hierarquia de Janelas do Shell do Windows
O Desktop do Windows não é uma superfície estática; trata-se de um controle Win32 padrão `SysListView32` hospedado em um container gerenciado pelo `explorer.exe`:

```text
Desktop Window (Progman ou WorkerW)
 └── SHELLDLL_DefView (Shell View Container)
      └── SysListView32 (Desktop ListView Control)
```

> **Atenção Técnica Crítica:**  
> Dependendo do estado da interface (como alternância de papéis de parede animados ou transições de temas), o container `SHELLDLL_DefView` pode migrar de `Progman` para uma janela dinâmica da classe `WorkerW`. A rotina de busca de identificadores de janela (`HWND`) **deve** lidar com essa contingência.

---

## 2. Restrições de Memória entre Processos (Cross-Process Memory)

O controle `SysListView32` reside no espaço de memória do processo `explorer.exe`. 

1. **Falha Típica (0xC0000005):** Passar ponteiros de buffers locais do processo Rust para mensagens `SendMessageW` (como `LVM_GETITEMTEXTW` ou `LVM_GETITEMPOSITION`) causará violação de acesso imediata.
2. **Requisito Obrigatório:** Toda leitura de dados dinâmicos requer alocação de memória no processo remoto do `explorer.exe`:
   * Obter `PID` via `GetWindowThreadProcessId`.
   * Obter handle com `OpenProcess(PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE, ...)`.
   * Alocar buffer remoto via `VirtualAllocEx(h_process, ..., MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE)`.
   * Escrever structs auxiliares remotamente com `WriteProcessMemory`.
   * Disparar mensagem via `SendMessageW`.
   * Recuperar conteúdo preenchido com `ReadProcessMemory`.
   * Desalocar o buffer remoto via `VirtualFreeEx(h_process, ..., 0, MEM_RELEASE)`.

---

## 3. Estruturas de Dados e Tipos

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconEntry {
    pub name: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopProfile {
    pub profile_name: String,
    pub resolution: Resolution,
    pub timestamp_utc: u64,
    pub icons: Vec<IconEntry>,
}
```

---

## 4. Pipeline de Desenvolvimento para o Modelo de IA

Ao solicitar código ou tarefas ao agente/modelo de IA, implemente de forma estritamente modular nas seguintes fases:

### Fase 1: Localizador de Handles (`find_desktop_listview`)
* Implementar busca recursiva/iterativa de janelas:
  1. `FindWindowW(w!("Progman"), None)`.
  2. Verificar se `SHELLDLL_DefView` é filha direta.
  3. Caso negativo, iterar pelas janelas irmãs `WorkerW` usando `FindWindowExW`.
  4. Localizar a janela neta `SysListView32`.

### Fase 2: Gerenciamento Seguro de Memória Remota (RAII)
* Não espalhar chamadas de `VirtualAllocEx`/`VirtualFreeEx` pela lógica de negócios.
* Criar uma struct `RemoteBuffer` em Rust que implementa `Drop` para garantir `VirtualFreeEx` mesmo em caso de erro (`panic` ou early return via `?`).

### Fase 3: Extração de Layout (`save_layout`)
* Obter quantidade total de ícones: `SendMessageW(hwnd, LVM_GETITEMCOUNT, 0, 0)`.
* Para cada índice `i` de `0..count`:
  * Enviar `LVM_GETITEMPOSITION` com o endereço do `RemoteBuffer` configurado para `POINT`.
  * Enviar `LVM_GETITEMTEXTW` com estrutura `LVITEMW` alocada remotamente.
  * Capturar resolução atual via `GetSystemMetrics(SM_CXSCREEN)` e `GetSystemMetrics(SM_CYSCREEN)`.
  * Persistir dados em JSON via `serde_json`.

### Fase 4: Restauração de Layout (`restore_layout`)
* Desabilitar auto-organização temporariamente:
  * Consultar estilos estendidos do ListView ou via COM `IFolderView2`.
* Mapear o nome de cada ícone presente no Desktop atual para seu respectivo índice.
* Para cada correspondência no perfil salvo:
  * Reposicionar com `SendMessageW(hwnd, LVM_SETITEMPOSITION, WPARAM(index), LPARAM(MAKELPARAM(x, y)))`.
* Invalidar área para forçar renderização: `InvalidateRect(hwnd, None, true)` e `RedrawWindow(...)`.

---

## 5. Diretrizes para o Modelo de Linguagem ao Gerar Código

* **Crates Obrigatórias:**
  * `windows = { version = "0.58", features = ["Win32_UI_WindowsAndMessaging", "Win32_System_Memory", "Win32_System_Threading", "Win32_Foundation", "Win32_Graphics_Gdi"] }`
  * `serde = { version = "1.0", features = ["derive"] }`
  * `serde_json = "1.0"`
* **Políticas de Concorrência e Segurança:**
  * Encapsular ponteiros brutos (`*mut c_void`, `HWND`) com verificações estritas de nulidade (`is_invalid()`, `is_null()`).
  * Sempre validar o retorno de `OpenProcess` e `VirtualAllocEx` antes de invocar `SendMessageW`.
  * Utilizar `String::from_utf16_lossy` para decodificar saídas wchar do Windows.