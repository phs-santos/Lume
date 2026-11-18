## O que tem no Lume VERSION

Uma máquina de escrever para livros, com tudo guardado cifrado no seu computador.

- **Escrever sem distração:** a linha em que você escreve fica sempre no mesmo lugar da tela, como no rolo de uma máquina de escrever. O texto salva sozinho a cada pausa, e cada cena guarda as versões antigas para voltar quando quiser.
- **Livro em capítulos e cenas:** o sumário à esquerda mostra o livro. O ⋯ de cada linha (ou o clique direito) cria uma cena antes ou depois, renomeia ou apaga; arrastar muda a ordem.
- **Imagens no texto:** cole ou arraste uma imagem para dentro da cena, com legenda.
- **Ficha do livro:** autor, sinopse, dedicatória e capa (a sua ou uma gerada pelo Lume).
- **Exportar:** EPUB com capa, sumário, imagens e contracapa; Word, PDF e Markdown, do livro inteiro ou só da cena aberta.
- **Assistente de IA (opcional):** conversa sobre o livro sem perder o fio, e a Bíblia guarda personagens, lugares e fatos que a IA encontra no texto. Dois modos: Conselheira (só opina) e Coautora (propõe trechos, que só entram se você aprovar). Mostra o custo de cada resposta e aceita um limite de gasto por mês. Precisa de uma chave da Inworld; sem ela, o resto funciona normalmente.
- **Ideias do celular:** anote ou grave áudios no celular e eles chegam aqui; os áudios viram texto. Um QR code abre o Lume no celular já com o seu nome.
- **Seu texto é só seu:** cofre cifrado, vários usuários no mesmo computador e 24 palavras de recuperação em português caso a senha seja esquecida.

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
