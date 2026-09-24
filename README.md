## DesktopORZ

Salve a possição dos icones na área de trabalho para que seja possível reeorganizar tudo com um único comando.

## Motivação para criar:

Criei esse projeto pois compartilho a área de trabalho do meu notebook com o meu computador via Google Drive. Sempre que eu alternava entre eles, a área de trabalho ficava uma bagunça. 

Para resolver isso, comecei usando o DesktopOK. Porém, por não ser open source e não saber quem é o desenvolvedor, decidi criar minha própria versão em Rust, aberta para a comunidade usar e contribuir.

### Como Utilizar:

Para utilizar o projeto você precisa ter o Rust instaldo no computador o qual pode ser instaldo por meio do site``https://rust-lang.org/tools/install/``

1. clonar o repositório:

   via GitHub:
   ```
   git clone https://github.com/ThainanViniciusKatchan/DesktopORZ.git
    ```
   via Codeberg:
   ```
   https://codeberg.org/ThainanViniciusKatchan/DesktopORZ.git
    ```
3. **Acesse a pasta do projeto:**
   
   CMD
   
   ```
   cd DesktopORZ
   ```

4. **Compile o projeto:**
   
   CMD
   
   ```
   cargo build --release
   ```
   
   *(O executável será gerado na pasta `target/release/` ou em ``C:\cargo-target\DesktopORZ\release``)*

## Comandos Principais

- **Ajuda:**
  
  ```
  DesktopORZ help
  ```
  
  Mostra todos os comandos, funções e como usar.



- **Salvar área de trabalho:**
  
  ```
  DesktopORZ save nome_do_perfil
  ```
  
  Salva a disposição atual dos ícones no perfil informado.



- **Restaurar área de trabalho:**
  
  CMD
  
  ```
  DesktopORZ restore nome_do_perfil
  ```
  
  Restaura os ícones para as posições salvas no perfil informado.
  

> Nota: No momento esses são os comandos principais via CLI, mas sigo implementando novas melhorias. Futuramente pretendo criar uma versão com interface gráfica (UI), mas não meu foco atual.

### Outros Comandos:

- ``DesktopORZ startup``
  
  - `on nome_do_perfil` A organização irá acontecer sempre que ligar o computador para o perfil desejado.
  
  - ``status`` Mostra se a inicialização com o sistema está ativado ou desativado.
  
  - ``off`` desliga a inicialização com o sistema.

- ``DesktopORZ save-res``
  
  - ``save-res nome_do_perfil`` Salva os ícones na resolução atual.
  
  - ``save-res nome_do_perfil LARGURAxALTURA`` salva os ícones na resolução passada, se caso precisar por exemplo, 1920x1080.



Projeto Brasileiro 🇧🇷


Desenvolvido com o apoio do modelo Kimi-K3 e agente Codebuff.
