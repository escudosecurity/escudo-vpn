#!/usr/bin/env python3
"""Blog post generator for Escudo VPN SEO strategy.

Generates blog posts from templates targeting high-volume keywords.
Posts are generated as static HTML using the same design system.

Usage:
    python3 blog_generator.py                    # Generate all posts
    python3 blog_generator.py --post phishing    # Generate one post
    python3 blog_generator.py --list             # List available posts
"""

import argparse
import os
from datetime import date
from pathlib import Path

from jinja2 import Environment, FileSystemLoader

from config import SITE_DIR, TEMPLATE_DIR, SITE_URL


BLOG_DIR = SITE_DIR / "blog"


def get_blog_posts():
    """Define all blog posts with SEO data."""
    today = date.today().isoformat()
    return [
        {
            "slug": "o-que-e-phishing",
            "title": "O que e Phishing? Como Identificar e se Proteger em 2026",
            "meta_title": "O que e Phishing? Guia Completo de Protecao | Escudo VPN",
            "meta_description": "Entenda o que e phishing, como identificar emails e sites falsos, e proteja seus dados. Guia pratico com exemplos reais e dicas de seguranca.",
            "target_keyword": "phishing o que e",
            "volume": 14800,
            "date": today,
            "category": "Seguranca",
            "sections": [
                {
                    "title": "O que e phishing?",
                    "content": """Phishing e um tipo de ataque cibernetico onde criminosos se passam por empresas ou pessoas confiaveis para roubar seus dados pessoais — senhas, numeros de cartao, CPF e credenciais bancarias.

O nome vem de "fishing" (pescar em ingles): os atacantes lancam uma "isca" (email, SMS ou site falso) e esperam que voce "morda" clicando no link ou fornecendo suas informacoes.

No Brasil, o phishing e a forma mais comum de fraude digital. Segundo dados da Febraban, tentativas de phishing cresceram 45% em 2025, com prejuizos de R$ 2,5 bilhoes."""
                },
                {
                    "title": "Como funciona um ataque de phishing",
                    "content": """Um ataque tipico segue estas etapas:

<strong>1. Contato inicial</strong> — Voce recebe um email, SMS ou mensagem no WhatsApp que parece ser do seu banco, da Receita Federal, dos Correios ou de uma loja online.

<strong>2. Urgencia fabricada</strong> — A mensagem cria pressao: "Sua conta sera bloqueada", "Compra nao reconhecida de R$ 3.500", "Seu CPF sera cancelado".

<strong>3. Link falso</strong> — Um botao ou link leva voce a um site identico ao original, mas com URL diferente (ex: banco-do-brasil.com.net em vez de bb.com.br).

<strong>4. Captura de dados</strong> — Ao digitar sua senha ou dados no site falso, tudo e enviado diretamente ao criminoso.

<strong>5. Uso imediato</strong> — Em minutos, o atacante acessa sua conta real, faz transferencias ou compras."""
                },
                {
                    "title": "Tipos de phishing mais comuns no Brasil",
                    "content": """<strong>Email phishing</strong> — Emails falsos de bancos, Netflix, Mercado Livre. Representam 65% dos ataques.

<strong>SMS phishing (smishing)</strong> — Mensagens como "Seus pontos expiram hoje" ou "Rastreie sua encomenda". Cresceu 120% em 2025.

<strong>WhatsApp phishing</strong> — Links maliciosos enviados por contatos hackeados ou numeros desconhecidos. Muito efetivo no Brasil por causa da popularidade do app.

<strong>Spear phishing</strong> — Ataques direcionados usando seus dados reais (nome, empresa, cargo) para parecer mais convincente.

<strong>Phishing por voz (vishing)</strong> — Ligacoes de falsos atendentes de banco pedindo confirmacao de dados."""
                },
                {
                    "title": "Como identificar phishing: 7 sinais",
                    "content": """<strong>1. Urgencia excessiva</strong> — "Acao imediata necessaria", "Sua conta sera encerrada em 24h".

<strong>2. Erros de portugues</strong> — Textos com erros gramaticais, acentuacao incorreta ou formatacao estranha.

<strong>3. URL suspeita</strong> — Passe o dedo (ou mouse) sobre o link ANTES de clicar. Se o dominio nao e o oficial, nao clique.

<strong>4. Remetente estranho</strong> — O email vem de "suporte@banco-seguro.xyz" em vez do dominio oficial.

<strong>5. Pedido de dados sensiveis</strong> — Nenhum banco ou empresa pede senha, CVV ou token por email/SMS.

<strong>6. Ofertas boas demais</strong> — "Voce ganhou um iPhone" ou "Resgate R$ 1.000 do FGTS".

<strong>7. Anexos inesperados</strong> — Boletos, comprovantes ou "fotos" que voce nao solicitou."""
                },
                {
                    "title": "Como se proteger contra phishing",
                    "content": """<strong>Nunca clique em links de emails/SMS suspeitos</strong> — Acesse o site digitando o endereco diretamente no navegador.

<strong>Ative autenticacao em dois fatores (2FA)</strong> — Mesmo que roubem sua senha, precisarao do segundo fator.

<strong>Use um DNS seguro com bloqueio de phishing</strong> — O Escudo VPN bloqueia automaticamente dominios conhecidos de phishing via DNS, antes mesmo do site carregar. Sao mais de 500 mil dominios maliciosos bloqueados.

<strong>Verifique o certificado SSL</strong> — Sites legitimos de bancos sempre tem HTTPS com cadeado. Mas atencao: sites falsos tambem podem ter HTTPS.

<strong>Mantenha apps atualizados</strong> — Atualizacoes corrigem vulnerabilidades que phishing pode explorar.

<strong>Use uma VPN em redes publicas</strong> — Em WiFi de shopping, aeroporto ou cafe, uma VPN impede que atacantes interceptem seus dados."""
                },
                {
                    "title": "O que fazer se voce caiu em phishing",
                    "content": """<strong>1. Troque suas senhas imediatamente</strong> — Comece pelo email e banco.

<strong>2. Ative 2FA em todas as contas</strong> — Use app autenticador, nao SMS.

<strong>3. Avise seu banco</strong> — Ligue para o SAC e solicite bloqueio preventivo.

<strong>4. Registre um B.O.</strong> — Faca boletim de ocorrencia online na Delegacia Virtual do seu estado.

<strong>5. Monitore seu CPF</strong> — Use o Registrato (Banco Central) para verificar contas abertas em seu nome.

<strong>6. Verifique vazamentos</strong> — Use nosso <a href="/vazamentos">verificador de vazamentos</a> para checar se seu email ja foi exposto."""
                },
            ],
            "faq": [
                ("O que e phishing e como funciona?", "Phishing e um golpe onde criminosos se passam por empresas confiaveis para roubar seus dados. Funciona atraves de emails, SMS ou sites falsos que imitam servicos reais."),
                ("Como saber se um email e phishing?", "Verifique o remetente, procure erros de portugues, nao clique em links suspeitos e desconfie de mensagens com urgencia excessiva."),
                ("VPN protege contra phishing?", "Uma VPN com DNS inteligente como o Escudo bloqueia dominios de phishing conhecidos automaticamente. Alem disso, criptografa seus dados para impedir interceptacao em redes publicas."),
                ("O que fazer se cliquei em link de phishing?", "Troque suas senhas imediatamente, ative autenticacao em dois fatores, avise seu banco e registre um boletim de ocorrencia."),
            ],
        },
        {
            "slug": "lgpd-o-que-e",
            "title": "LGPD: O que e a Lei Geral de Protecao de Dados e Seus Direitos",
            "meta_title": "LGPD: O que e, Como Funciona e Seus Direitos | Escudo VPN",
            "meta_description": "Entenda a LGPD (Lei 13.709/2018): o que sao dados pessoais, quais sao seus direitos e como empresas devem proteger suas informacoes. Guia completo.",
            "target_keyword": "lgpd o que e",
            "volume": 12100,
            "date": today,
            "category": "Privacidade",
            "sections": [
                {
                    "title": "O que e a LGPD?",
                    "content": """A LGPD (Lei Geral de Protecao de Dados Pessoais) e a lei brasileira que regulamenta como empresas e organizacoes podem coletar, armazenar e usar seus dados pessoais.

Criada pela Lei n° 13.709 de 14 de agosto de 2018, a LGPD entrou em vigor em setembro de 2020. E a versao brasileira do GDPR europeu e representa um marco na protecao da privacidade digital no Brasil.

A lei se aplica a qualquer empresa — brasileira ou estrangeira — que processe dados de pessoas no Brasil, independente de onde os servidores estejam localizados."""
                },
                {
                    "title": "O que sao dados pessoais segundo a LGPD?",
                    "content": """A LGPD define dois tipos de dados:

<strong>Dados pessoais</strong> — Qualquer informacao que identifique ou possa identificar uma pessoa: nome, CPF, RG, email, telefone, endereco IP, localizacao GPS, cookies de navegacao.

<strong>Dados sensiveis</strong> — Dados que exigem protecao extra: origem racial/etnica, convicao religiosa, opiniao politica, dados de saude, vida sexual, dados geneticos e biometricos.

Importante: ate mesmo seu endereco IP e considerado dado pessoal pela LGPD. Isso significa que sites que rastreiam seu IP sem consentimento estao violando a lei."""
                },
                {
                    "title": "Quais sao seus direitos pela LGPD?",
                    "content": """Voce tem direito a:

<strong>1. Confirmacao e acesso</strong> — Saber se uma empresa tem seus dados e solicitar uma copia.

<strong>2. Correcao</strong> — Pedir para corrigir dados incompletos ou desatualizados.

<strong>3. Anonimizacao ou exclusao</strong> — Solicitar que seus dados sejam apagados ou anonimizados.

<strong>4. Portabilidade</strong> — Transferir seus dados para outro fornecedor.

<strong>5. Revogacao do consentimento</strong> — Retirar sua autorizacao a qualquer momento.

<strong>6. Informacao sobre compartilhamento</strong> — Saber com quais terceiros seus dados foram compartilhados.

<strong>7. Oposicao</strong> — Recusar o tratamento de dados em certas situacoes."""
                },
                {
                    "title": "Como a LGPD afeta seu dia a dia",
                    "content": """Na pratica, a LGPD muda a forma como voce interage com servicos digitais:

<strong>Consentimento explicito</strong> — Sites precisam pedir sua permissao antes de coletar cookies e dados. Aqueles banners de cookies existem por causa da LGPD.

<strong>Politica de privacidade clara</strong> — Empresas devem explicar em linguagem simples o que fazem com seus dados.

<strong>Direito de deletar conta</strong> — Voce pode pedir para qualquer servico apagar completamente seus dados.

<strong>Notificacao de vazamentos</strong> — Empresas sao obrigadas a informar a ANPD e os afetados em caso de vazamento de dados.

<strong>Multas pesadas</strong> — Empresas que violam a LGPD podem ser multadas em ate 2% do faturamento, limitado a R$ 50 milhoes por infracao."""
                },
                {
                    "title": "Como proteger seus dados na pratica",
                    "content": """<strong>Use uma VPN</strong> — Seu endereco IP e dado pessoal pela LGPD. O Escudo VPN mascara seu IP e criptografa todo o trafego, impedindo rastreamento.

<strong>Revise permissoes de apps</strong> — Muitos apps pedem acesso a localizacao, contatos e camera sem necessidade.

<strong>Use email descartavel</strong> — Para cadastros em sites que voce nao confia, use aliases de email.

<strong>Ative bloqueio de rastreadores</strong> — O Escudo Shield bloqueia rastreadores de terceiros automaticamente via DNS.

<strong>Verifique vazamentos regularmente</strong> — Use nosso <a href="/vazamentos">verificador de vazamentos</a> para saber se seus dados foram expostos.

<strong>Exercite seus direitos</strong> — Se uma empresa nao responder sua solicitacao em 15 dias, denuncie a ANPD (anpd.gov.br)."""
                },
            ],
            "faq": [
                ("O que e LGPD resumido?", "A LGPD e a lei brasileira que protege seus dados pessoais, regulamentando como empresas coletam, armazenam e usam suas informacoes."),
                ("Quais dados a LGPD protege?", "Todos os dados que identificam voce: nome, CPF, email, telefone, endereco IP, localizacao, cookies e dados biometricos."),
                ("O que acontece se uma empresa violar a LGPD?", "Multas de ate 2% do faturamento (maximo R$ 50 milhoes), alem de obrigacao de corrigir e notificar os afetados."),
            ],
        },
        {
            "slug": "vazamento-de-dados",
            "title": "Vazamento de Dados: Como Saber se Voce foi Afetado e o que Fazer",
            "meta_title": "Vazamento de Dados: Verifique se Seus Dados Vazaram | Escudo VPN",
            "meta_description": "Descubra se seus dados pessoais foram vazados. Saiba como verificar, o que fazer apos um vazamento e como proteger suas informacoes.",
            "target_keyword": "vazamento de dados",
            "volume": 8100,
            "date": today,
            "category": "Seguranca",
            "sections": [
                {
                    "title": "O que e um vazamento de dados?",
                    "content": """Vazamento de dados ocorre quando informacoes pessoais armazenadas por empresas sao expostas sem autorizacao — seja por ataque hacker, falha de seguranca ou erro humano.

No Brasil, os maiores vazamentos recentes incluem a exposicao de 223 milhoes de CPFs em 2021 e diversos incidentes envolvendo operadoras, bancos e orgaos publicos.

Dados vazados incluem: CPF, nome completo, email, telefone, endereco, dados bancarios, senhas e ate fotos de documentos."""
                },
                {
                    "title": "Como verificar se seus dados vazaram",
                    "content": """<strong>1. Use nosso verificador gratuito</strong> — Acesse <a href="/vazamentos">escudovpn.com/vazamentos</a> e digite seu email. Verificamos instantaneamente no banco de dados Have I Been Pwned, que cataloga mais de 13 bilhoes de contas vazadas.

<strong>2. Registrato do Banco Central</strong> — Em registrato.bcb.gov.br voce consulta contas bancarias, emprestimos e chaves Pix vinculadas ao seu CPF.

<strong>3. Google Password Checkup</strong> — Em passwords.google.com verifique se senhas salvas no Chrome foram comprometidas.

<strong>4. Serasa</strong> — O site da Serasa oferece monitoramento basico gratuito de CPF."""
                },
                {
                    "title": "O que fazer se seus dados vazaram",
                    "content": """<strong>1. Troque todas as senhas</strong> — Comece pelo email principal, banco e redes sociais. Use senhas unicas para cada servico.

<strong>2. Ative 2FA em tudo</strong> — Autenticacao em dois fatores e sua melhor defesa. Prefira app autenticador (Google Authenticator, Authy) ao SMS.

<strong>3. Monitore suas contas bancarias</strong> — Verifique extratos diariamente por 30 dias. Ative alertas de transacao.

<strong>4. Congele seu credito</strong> — Contate Serasa e SPC para bloquear consultas ao seu CPF temporariamente.

<strong>5. Registre B.O.</strong> — Faca boletim de ocorrencia online. Isso protege voce juridicamente.

<strong>6. Proteja sua conexao</strong> — Use o Escudo VPN para criptografar seu trafego e impedir novas interceptacoes. O Escudo Shield bloqueia sites maliciosos que tentam explorar dados vazados."""
                },
                {
                    "title": "Como prevenir vazamentos futuros",
                    "content": """<strong>Senhas unicas</strong> — Use um gerenciador de senhas. Nunca reutilize senhas entre servicos.

<strong>VPN sempre ativa</strong> — Especialmente em redes WiFi publicas, onde seus dados podem ser interceptados.

<strong>Minimize dados compartilhados</strong> — Nao forneca CPF, telefone ou endereco quando nao for necessario.

<strong>DNS com bloqueio de ameacas</strong> — O Escudo VPN filtra dominios maliciosos conhecidos, impedindo que seu dispositivo se conecte a servidores comprometidos.

<strong>Revise apps e permissoes</strong> — Desinstale apps que voce nao usa. Revise permissoes de localizacao e contatos."""
                },
            ],
            "faq": [
                ("Como saber se meu CPF vazou?", "Use nosso verificador em escudovpn.com/vazamentos com seu email, e consulte o Registrato do Banco Central para verificar movimentacoes suspeitas."),
                ("O que hackers fazem com dados vazados?", "Vendem na dark web, usam para fraudes bancarias, abrem contas em seu nome, aplicam golpes de phishing direcionados ou fazem extorsao."),
                ("VPN protege contra vazamento de dados?", "Uma VPN protege seus dados em transito (na rede), impedindo interceptacao. Porem, se uma empresa que tem seus dados for hackeada, a VPN nao impede esse vazamento no servidor deles."),
            ],
        },
        {
            "slug": "dns-privado",
            "title": "DNS Privado: O que e, Como Configurar e Por que Usar",
            "meta_title": "DNS Privado: Guia Completo de Configuracao | Escudo VPN",
            "meta_description": "Entenda o que e DNS privado, como configurar no Android e iOS, e por que usar um DNS seguro protege sua privacidade e bloqueia anuncios.",
            "target_keyword": "dns privado",
            "volume": 5400,
            "date": today,
            "category": "Privacidade",
            "sections": [
                {
                    "title": "O que e DNS e por que importa?",
                    "content": """DNS (Domain Name System) e o sistema que traduz nomes de sites em enderecos IP. Quando voce digita "google.com", o DNS converte para 142.250.80.46.

Por padrao, seu provedor de internet (Vivo, Claro, TIM) controla seu DNS. Isso significa que eles podem ver TODOS os sites que voce acessa, mesmo que voce use HTTPS.

Alem disso, provedores podem redirecionar suas consultas DNS para exibir paginas de erro com anuncios, ou ate bloquear sites por ordem judicial."""
                },
                {
                    "title": "O que e DNS privado?",
                    "content": """DNS privado e um servidor DNS que criptografa suas consultas, impedindo que seu provedor de internet, governo ou qualquer pessoa na rede veja quais sites voce acessa.

Protocolos de DNS privado incluem:

<strong>DNS over HTTPS (DoH)</strong> — Envia consultas DNS dentro de conexoes HTTPS. Dificil de bloquear.

<strong>DNS over TLS (DoT)</strong> — Criptografa consultas na porta 853. O Android nativo suporta.

<strong>DNSCrypt</strong> — Protocolo aberto com autenticacao e criptografia."""
                },
                {
                    "title": "DNS privado vs VPN: qual a diferenca?",
                    "content": """<strong>DNS privado</strong> protege apenas suas consultas DNS — quais sites voce acessa. Seu trafego real (conteudo das paginas) continua visivel.

<strong>VPN</strong> criptografa TODO o trafego — DNS, conteudo, downloads, streaming. Ninguem ve nada.

O ideal e usar os dois. O Escudo VPN combina VPN + DNS privado com bloqueio de ameacas: criptografa todo o trafego E filtra dominios maliciosos, anuncios e rastreadores automaticamente."""
                },
                {
                    "title": "Como configurar DNS privado no Android",
                    "content": """<strong>1.</strong> Abra Configuracoes > Rede e Internet > DNS Privado

<strong>2.</strong> Selecione "Nome do host do provedor de DNS privado"

<strong>3.</strong> Digite um dos seguintes:
- <code>dns.adguard-dns.com</code> (bloqueia anuncios)
- <code>cloudflare-dns.com</code> (rapido, sem bloqueio)
- <code>dns.google</code> (Google, sem bloqueio)

<strong>4.</strong> Toque Salvar.

Ou melhor: instale o Escudo VPN e tenha DNS privado + bloqueio de anuncios + criptografia completa automaticamente, sem configuracao manual."""
                },
                {
                    "title": "Beneficios do DNS com bloqueio de ameacas",
                    "content": """O Escudo Shield vai alem do DNS privado — ele filtra ativamente:

<strong>500.000+ dominios bloqueados</strong> — Anuncios, malware, phishing e rastreadores nunca chegam ao seu dispositivo.

<strong>4 feeds de ameacas atualizados diariamente</strong> — HaGezi, URLhaus, PhishingFilter e IP Blacklists.

<strong>Bloqueio no nivel do DNS</strong> — Mais eficiente que extensoes de navegador. Funciona em todos os apps, nao so no browser.

<strong>Zero logs</strong> — Nao registramos quais sites voce acessa."""
                },
            ],
            "faq": [
                ("DNS privado e seguro?", "Sim, DNS privado criptografa suas consultas DNS, impedindo que seu provedor de internet veja quais sites voce acessa."),
                ("DNS privado deixa a internet lenta?", "Nao. Na maioria dos casos, um bom DNS privado e mais rapido que o DNS do seu provedor."),
                ("Qual o melhor DNS privado para Android?", "Para bloqueio de anuncios, use dns.adguard-dns.com. Para protecao completa, use o Escudo VPN que combina DNS privado com VPN e bloqueio de ameacas."),
            ],
        },
        {
            "slug": "whatsapp-clonado",
            "title": "WhatsApp Clonado: Como Saber, Prevenir e Recuperar sua Conta",
            "meta_title": "WhatsApp Clonado: Como Prevenir e Recuperar | Escudo VPN",
            "meta_description": "Descubra se seu WhatsApp foi clonado, como prevenir a clonagem e o que fazer para recuperar sua conta. Guia completo de seguranca.",
            "target_keyword": "whatsapp clonado",
            "volume": 3600,
            "date": today,
            "category": "Seguranca",
            "sections": [
                {
                    "title": "Como saber se seu WhatsApp foi clonado",
                    "content": """<strong>Sinais de clonagem:</strong>

<strong>1. Sessoes ativas desconhecidas</strong> — Va em WhatsApp > Dispositivos conectados. Se houver sessoes que voce nao reconhece, seu WhatsApp pode estar comprometido.

<strong>2. Mensagens que voce nao enviou</strong> — Contatos recebem mensagens suas pedindo dinheiro ou com links estranhos.

<strong>3. Codigo de verificacao nao solicitado</strong> — Receber SMS com codigo de 6 digitos do WhatsApp sem ter pedido.

<strong>4. Desconexoes inexplicaveis</strong> — Se o WhatsApp pedir para verificar seu numero repentinamente.

<strong>5. Consumo de dados anormal</strong> — Aumento no uso de dados moveis sem motivo aparente."""
                },
                {
                    "title": "Como criminosos clonam WhatsApp",
                    "content": """<strong>Engenharia social</strong> — O metodo mais comum. O criminoso liga fingindo ser de uma empresa e pede o codigo de 6 digitos que voce recebeu por SMS. Com esse codigo, ele ativa seu WhatsApp em outro celular.

<strong>SIM swap</strong> — O criminoso convence a operadora a transferir seu numero para outro chip. Assim recebe todos seus SMS, incluindo codigos de verificacao.

<strong>WhatsApp Web</strong> — Se alguem tem acesso fisico ao seu celular por alguns segundos, pode escanear o QR code do WhatsApp Web e ler todas suas mensagens.

<strong>Malware</strong> — Apps maliciosos instalados no celular podem capturar o codigo de verificacao ou espelhar o WhatsApp."""
                },
                {
                    "title": "Como prevenir a clonagem",
                    "content": """<strong>1. Ative verificacao em duas etapas</strong> — WhatsApp > Configuracoes > Conta > Verificacao em duas etapas. Crie um PIN de 6 digitos. Isso impede que alguem ative seu numero sem o PIN.

<strong>2. Nunca compartilhe codigos</strong> — NENHUMA empresa legitima pede o codigo de verificacao do WhatsApp. Nunca.

<strong>3. Desconecte sessoes desconhecidas</strong> — Revise Dispositivos conectados regularmente.

<strong>4. Use uma VPN em redes publicas</strong> — O Escudo VPN impede interceptacao de dados em WiFi publico, dificultando ataques man-in-the-middle.

<strong>5. Proteja seu chip</strong> — Coloque um PIN no SIM card (Configuracoes > Seguranca > Bloqueio do SIM).

<strong>6. Cuidado com links</strong> — O Escudo Shield bloqueia dominios maliciosos enviados via WhatsApp antes de voce clicar."""
                },
                {
                    "title": "O que fazer se seu WhatsApp foi clonado",
                    "content": """<strong>1. Reinstale o WhatsApp</strong> — Desinstale e reinstale o app. Ao verificar seu numero, o criminoso sera desconectado.

<strong>2. Ative verificacao em duas etapas</strong> — Imediatamente apos recuperar a conta.

<strong>3. Avise seus contatos</strong> — Publique nos stories e avise grupos que sua conta foi comprometida. Peca para ignorarem mensagens suspeitas.

<strong>4. Registre B.O.</strong> — Faca boletim de ocorrencia online. Golpe via WhatsApp e crime (estelionato digital, Art. 171 CP).

<strong>5. Contate a operadora</strong> — Se suspeitar de SIM swap, peca bloqueio e troca do chip.

<strong>6. Verifique seus dados</strong> — Use nosso <a href="/vazamentos">verificador de vazamentos</a> para checar se mais dados foram expostos."""
                },
            ],
            "faq": [
                ("Como saber se meu WhatsApp foi clonado?", "Verifique Dispositivos conectados no WhatsApp. Se houver sessoes desconhecidas, ou se contatos recebem mensagens que voce nao enviou, sua conta pode estar comprometida."),
                ("Como ativar verificacao em duas etapas no WhatsApp?", "Va em Configuracoes > Conta > Verificacao em duas etapas > Ativar. Crie um PIN de 6 digitos."),
                ("VPN protege o WhatsApp?", "Uma VPN criptografa todo o trafego do WhatsApp em redes publicas, impedindo interceptacao. O DNS do Escudo tambem bloqueia links maliciosos recebidos no chat."),
            ],
        },
        {
            "slug": "seguranca-digital-guia",
            "title": "Seguranca Digital: Guia Completo para Proteger sua Vida Online",
            "meta_title": "Seguranca Digital: Guia Completo 2026 | Escudo VPN",
            "meta_description": "Guia completo de seguranca digital: como proteger senhas, dados pessoais, celular e privacidade online. Dicas praticas para 2026.",
            "target_keyword": "seguranca digital",
            "volume": 3600,
            "date": today,
            "category": "Seguranca",
            "sections": [
                {
                    "title": "O que e seguranca digital?",
                    "content": """Seguranca digital e o conjunto de praticas, ferramentas e habitos que protegem seus dados, dispositivos e privacidade no mundo online.

Com mais de 150 milhoes de brasileiros conectados, a seguranca digital deixou de ser preocupacao so de empresas — e questao de sobrevivencia digital pessoal.

O <a href="/seguranca-digital/">Indice de Seguranca Digital</a> do Escudo VPN analisa a vulnerabilidade de mais de 5.500 municipios brasileiros, considerando velocidade de internet, cobertura de rede movel, indicadores de seguranca e infraestrutura digital."""
                },
                {
                    "title": "Os 5 pilares da seguranca digital pessoal",
                    "content": """<strong>1. Senhas fortes e unicas</strong> — Use um gerenciador de senhas. Cada conta deve ter senha diferente com pelo menos 12 caracteres.

<strong>2. Autenticacao em dois fatores (2FA)</strong> — Ative em todas as contas: email, banco, redes sociais. Prefira app autenticador ao SMS.

<strong>3. Criptografia de trafego</strong> — Use VPN para criptografar toda sua conexao. O Escudo VPN usa criptografia pos-quantica que protege ate contra computadores quanticos futuros.

<strong>4. Bloqueio de ameacas</strong> — DNS inteligente que bloqueia phishing, malware e rastreadores antes de chegarem ao seu dispositivo.

<strong>5. Atualizacoes em dia</strong> — Mantenha sistema operacional e apps atualizados. Atualizacoes corrigem vulnerabilidades exploradas por atacantes."""
                },
                {
                    "title": "Seguranca digital no celular",
                    "content": """Seu celular e o dispositivo mais vulneravel — contem banco, email, WhatsApp, fotos e documentos.

<strong>Bloqueio de tela</strong> — Use biometria + senha longa. Nunca use padrao de desbloqueio.

<strong>Permissoes de apps</strong> — Revise e revogue permissoes desnecessarias de localizacao, camera e microfone.

<strong>WiFi publico</strong> — Nunca acesse banco ou faca compras em WiFi publico sem VPN.

<strong>Bloqueio de anuncios no celular</strong> — Anuncios maliciosos (malvertising) podem infectar seu celular. O Escudo Shield bloqueia mais de 500 mil dominios de anuncios e malware.

<strong>Backup criptografado</strong> — Ative backup criptografado do WhatsApp e das fotos."""
                },
                {
                    "title": "Ferramentas essenciais de seguranca digital",
                    "content": """<strong>VPN (Rede Privada Virtual)</strong> — Criptografa todo o trafego. O Escudo VPN oferece criptografia pos-quantica + bloqueio de ameacas + zero logs.

<strong>Gerenciador de senhas</strong> — Bitwarden (gratuito) ou 1Password. Gera e armazena senhas unicas.

<strong>Autenticador 2FA</strong> — Google Authenticator, Authy ou Microsoft Authenticator.

<strong>Verificador de vazamentos</strong> — <a href="/vazamentos">escudovpn.com/vazamentos</a> verifica se seu email foi exposto em vazamentos conhecidos.

<strong>Teste de velocidade</strong> — <a href="/teste-de-velocidade">escudovpn.com/teste-de-velocidade</a> mostra sua velocidade real e compara com a media da sua cidade."""
                },
            ],
            "faq": [
                ("O que e seguranca digital?", "Seguranca digital e o conjunto de praticas e ferramentas que protegem seus dados, dispositivos e privacidade online."),
                ("Qual a importancia da seguranca digital?", "Com dados bancarios, documentos e comunicacoes no celular, uma falha de seguranca pode resultar em prejuizo financeiro, roubo de identidade e exposicao de informacoes privadas."),
                ("Como melhorar minha seguranca digital?", "Use senhas unicas com gerenciador, ative 2FA, instale uma VPN como o Escudo, mantenha apps atualizados e cuidado com links suspeitos."),
            ],
        },
    ]


def setup_jinja() -> Environment:
    return Environment(
        loader=FileSystemLoader(str(TEMPLATE_DIR)),
        autoescape=False,
        trim_blocks=True,
        lstrip_blocks=True,
    )


def render_blog_post(env: Environment, post: dict) -> str:
    template = env.get_template("blog_post.html")
    return template.render(post=post)


def render_blog_index(env: Environment, posts: list) -> str:
    template = env.get_template("blog_index.html")
    return template.render(posts=posts)


def generate_blog_sitemap(posts: list):
    today = date.today().isoformat()
    lines = ['<?xml version="1.0" encoding="UTF-8"?>']
    lines.append('<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">')
    lines.append(f'  <url><loc>{SITE_URL}/blog/</loc><lastmod>{today}</lastmod></url>')
    for post in posts:
        lines.append(f'  <url><loc>{SITE_URL}/blog/{post["slug"]}</loc><lastmod>{post["date"]}</lastmod></url>')
    lines.append('</urlset>')

    sitemap = SITE_DIR / "sitemap-blog.xml"
    sitemap.write_text("\n".join(lines), encoding="utf-8")
    print(f"  {sitemap} ({len(posts) + 1} URLs)")

    # Update sitemap index
    sitemap_index = SITE_DIR / "sitemap-index.xml"
    content = sitemap_index.read_text()
    if "sitemap-blog.xml" not in content:
        content = content.replace(
            "</sitemapindex>",
            f'  <sitemap><loc>{SITE_URL}/sitemap-blog.xml</loc><lastmod>{today}</lastmod></sitemap>\n</sitemapindex>'
        )
        sitemap_index.write_text(content, encoding="utf-8")
        print(f"  Updated {sitemap_index}")


def main():
    parser = argparse.ArgumentParser(description="Generate blog posts")
    parser.add_argument("--post", type=str, help="Generate only this post (slug)")
    parser.add_argument("--list", action="store_true", help="List available posts")
    args = parser.parse_args()

    posts = get_blog_posts()

    if args.list:
        for p in posts:
            print(f"  {p['slug']:<30} {p['target_keyword']:<25} {p['volume']:>6}/mo")
        return

    env = setup_jinja()

    BLOG_DIR.mkdir(parents=True, exist_ok=True)

    if args.post:
        posts = [p for p in posts if p["slug"] == args.post]
        if not posts:
            print(f"Post '{args.post}' not found")
            return

    print(f"Generating {len(posts)} blog posts...")
    for post in posts:
        out_path = BLOG_DIR / f"{post['slug']}.html"
        html = render_blog_post(env, post)
        out_path.write_text(html, encoding="utf-8")
        print(f"  {post['slug']} ({post['volume']}/mo)")

    # Generate index
    all_posts = get_blog_posts()
    out_path = BLOG_DIR / "index.html"
    html = render_blog_index(env, all_posts)
    out_path.write_text(html, encoding="utf-8")
    print("  Blog index generated")

    # Sitemap
    print("Generating blog sitemap...")
    generate_blog_sitemap(all_posts)

    print(f"\nDone! {len(posts)} blog posts in {BLOG_DIR}")


if __name__ == "__main__":
    main()
