## O que mudou na VERSION

- **Mapa de ideias:** cada livro ganhou um mapa de cartões ligados por setas, para organizar personagens, cenas e viradas antes de escrever. Dois cliques criam um cartão, já pronto para digitar; puxar a bolinha de um cartão até outro cria uma seta, que pode ter um rótulo. Os cartões têm cinco cores e podem virar ficha de personagem, lugar ou fato na Bíblia.
- **Texto ou mapa:** no editor, a aba Texto | Mapa troca entre a folha e o mapa do livro. O atalho é Ctrl+Shift+M (Cmd+Shift+M no Mac).
- **Começar pela ideia:** ao criar um livro, escolha entre escrever direto ou contar a ideia para a IA. Ela faz uma pergunta por vez sobre a história e vai montando o mapa com você, sem escrever o livro por você. Precisa da IA configurada.
- **Nome provisório:** dá para criar um livro sem nome. Ele aparece como "Livro sem nome", com a marca de nome provisório na estante, até você dar um nome ou aceitar um sugerido.
- **A IA sugere, você decide:** na conversa, a IA pode sugerir cartões e setas para o mapa e, ao desenvolver a ideia, um nome para o livro. Você vê cada cartão e cada ligação, escolhe quais entram com "Pôr no mapa" ou descarta tudo. Nada muda no mapa ou no nome sem a sua aprovação.
- **Instalar no Mac pelo Terminal:** o comando de instalação agora funciona direto, sem precisar de conta no GitHub.
- **Correções:** um livro recém-criado não pede mais para a IA ler cenas que ainda estão vazias.

## Baixar

Baixe só o arquivo do seu sistema:

| Sistema | Arquivo |
| --- | --- |
| Windows | [Lume_VERSION_x64-setup.exe](https://github.com/phs-santos/Lume/releases/download/vVERSION/Lume_VERSION_x64-setup.exe) |
| macOS com chip Apple (M1 ou mais novo) | [Lume_VERSION_aarch64.dmg](https://github.com/phs-santos/Lume/releases/download/vVERSION/Lume_VERSION_aarch64.dmg) |
| macOS (Intel) | [Lume_VERSION_x64.dmg](https://github.com/phs-santos/Lume/releases/download/vVERSION/Lume_VERSION_x64.dmg) |
| Linux (Debian, Ubuntu, Mint) | [Lume_VERSION_amd64.deb](https://github.com/phs-santos/Lume/releases/download/vVERSION/Lume_VERSION_amd64.deb) |
| Linux (outras distribuições) | [Lume_VERSION_amd64.AppImage](https://github.com/phs-santos/Lume/releases/download/vVERSION/Lume_VERSION_amd64.AppImage) |

**No Mac, o jeito mais fácil é pelo Terminal**, que não mostra o aviso da Apple. O comando escolhe a versão certa para o seu chip, confere o arquivo e coloca o Lume em Aplicativos:

```bash
curl -fsSL https://raw.githubusercontent.com/phs-santos/Lume/main/scripts/install-macos.sh | bash
```

No Mac, se for usar o `.dmg`, veja o chip em **menu Apple → Sobre Este Mac**: "Apple M…" usa o `aarch64`; "Intel" usa o `x64`.

Os outros arquivos da lista abaixo não são para baixar à mão:

- **Source code (zip / tar.gz):** é o código do projeto, que o GitHub anexa sozinho. Não é o app.
- **`.msi` do Windows:** o mesmo app do `.exe`, para instalação gerenciada por empresas.
- **`.app.tar.gz` do macOS e `.rpm`:** formatos alternativos (o `.rpm` é para Fedora e openSUSE).
- **`SHA256SUMS.txt`:** as somas de conferência dos arquivos, usadas pelo instalador do Mac.

## Antes de instalar

Os instaladores ainda não têm a assinatura paga da Apple e da Microsoft, então o sistema avisa na primeira vez:

- **Windows:** no aviso do SmartScreen, clique em **Mais informações → Executar assim mesmo**. Se aparecer "O controle inteligente de aplicativos bloqueou um aplicativo", não há botão para liberar: esse recurso do Windows 11 só deixa rodar instalador assinado.
- **macOS, pelo `.dmg`:** arraste o Lume para Aplicativos, abra uma vez e, em **Ajustes do Sistema → Privacidade e Segurança**, clique em **Abrir Mesmo Assim**. Pelo comando do Terminal acima, esse aviso não aparece.
- **Linux, `.deb`:** instale com `sudo apt install ./Lume_VERSION_amd64.deb`.
- **Linux, AppImage:** dê permissão de execução (`chmod +x Lume_VERSION_amd64.AppImage`) e abra.

Na primeira vez, o Lume pede para criar o cofre: um nome, uma senha e as 24 palavras de recuperação. Guarde as palavras num lugar seguro, fora do computador.
