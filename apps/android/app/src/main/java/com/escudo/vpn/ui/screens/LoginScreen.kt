package com.escudo.vpn.ui.screens

import android.Manifest
import android.widget.Toast
import android.webkit.JavascriptInterface
import android.webkit.WebChromeClient
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Visibility
import androidx.compose.material.icons.filled.VisibilityOff
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.IconButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.compose.ui.window.Dialog
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.escudo.vpn.data.repository.AuthRepository
import com.escudo.vpn.ui.theme.Accent
import com.escudo.vpn.ui.theme.Background
import com.escudo.vpn.ui.theme.CardBackground
import com.escudo.vpn.ui.theme.TextPrimary
import com.escudo.vpn.ui.theme.TextSecondary
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import javax.inject.Inject

@HiltViewModel
class LoginViewModel @Inject constructor(
    private val authRepository: AuthRepository
) : ViewModel() {

    private val _isLoading = MutableStateFlow(false)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    private val _loginSuccess = MutableSharedFlow<Unit>()
    val loginSuccess: SharedFlow<Unit> = _loginSuccess.asSharedFlow()

    private val _createdAccountNumber = MutableStateFlow<String?>(null)
    val createdAccountNumber: StateFlow<String?> = _createdAccountNumber.asStateFlow()

    fun login(email: String, password: String) {
        viewModelScope.launch {
            _isLoading.value = true
            _error.value = null
            val result = authRepository.login(email, password)
            _isLoading.value = false
            result.fold(
                onSuccess = { _loginSuccess.emit(Unit) },
                onFailure = { _error.value = it.message ?: "Erro ao fazer login" }
            )
        }
    }

    fun register(email: String, password: String) {
        viewModelScope.launch {
            _isLoading.value = true
            _error.value = null
            val result = authRepository.register(email, password)
            _isLoading.value = false
            result.fold(
                onSuccess = { _loginSuccess.emit(Unit) },
                onFailure = { _error.value = it.message ?: "Erro ao criar conta" }
            )
        }
    }

    fun loginWithNumber(accountNumber: String) {
        viewModelScope.launch {
            _isLoading.value = true
            _error.value = null
            val result = authRepository.loginWithNumber(accountNumber)
            _isLoading.value = false
            result.fold(
                onSuccess = { _loginSuccess.emit(Unit) },
                onFailure = { _error.value = it.message ?: "Erro ao entrar com código" }
            )
        }
    }

    fun createAnonymousAccount() {
        viewModelScope.launch {
            _isLoading.value = true
            _error.value = null
            val result = authRepository.createAnonymousAccountAndLogin()
            _isLoading.value = false
            result.fold(
                onSuccess = {
                    _createdAccountNumber.value = it
                    _loginSuccess.emit(Unit)
                },
                onFailure = { _error.value = it.message ?: "Erro ao gerar código da conta" }
            )
        }
    }

    fun scanQr(rawValue: String) {
        viewModelScope.launch {
            _isLoading.value = true
            _error.value = null
            val result = authRepository.scanQrToken(rawValue)
            _isLoading.value = false
            result.fold(
                onSuccess = { _loginSuccess.emit(Unit) },
                onFailure = { _error.value = it.message ?: "Erro ao escanear QR" }
            )
        }
    }
}

@Composable
fun LoginScreen(
    onLoginSuccess: () -> Unit,
    viewModel: LoginViewModel = hiltViewModel()
) {
    val context = LocalContext.current
    val clipboardManager = LocalClipboardManager.current
    var email by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var accountNumber by remember { mutableStateOf("") }
    var isRegisterMode by remember { mutableStateOf(false) }
    var useAccountNumber by remember { mutableStateOf(false) }
    var passwordVisible by remember { mutableStateOf(false) }
    var showQrScanner by remember { mutableStateOf(false) }

    val isLoading by viewModel.isLoading.collectAsState()
    val error by viewModel.error.collectAsState()
    val createdAccountNumber by viewModel.createdAccountNumber.collectAsState()
    val cameraPermissionLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.RequestPermission()
    ) { granted ->
        if (granted) {
            showQrScanner = true
        } else {
            Toast.makeText(context, "Permita a camera para escanear o QR", Toast.LENGTH_SHORT).show()
        }
    }

    LaunchedEffect(Unit) {
        viewModel.loginSuccess.collect {
            onLoginSuccess()
        }
    }

    if (showQrScanner) {
        QrScannerDialog(
            onDismiss = { showQrScanner = false },
            onScan = { rawValue ->
                showQrScanner = false
                viewModel.scanQr(rawValue)
            }
        )
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(Background)
            .padding(horizontal = 24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Text(
            text = "Escudo",
            fontWeight = FontWeight.Black,
            fontSize = 36.sp,
            color = TextPrimary,
            letterSpacing = (-0.03).em
        )

        Spacer(modifier = Modifier.height(16.dp))

        Text(
            text = if (useAccountNumber) {
                "Entre com seu código de 16 dígitos"
            } else if (isRegisterMode) {
                "Crie sua conta"
            } else {
                "Entre na sua conta"
            },
            style = MaterialTheme.typography.bodyMedium,
            color = TextSecondary
        )

        Spacer(modifier = Modifier.height(40.dp))

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            ModeChip(
                label = "E-mail",
                selected = !useAccountNumber,
                onClick = { useAccountNumber = false },
                modifier = Modifier.weight(1f)
            )
            ModeChip(
                label = "Código",
                selected = useAccountNumber,
                onClick = { useAccountNumber = true },
                modifier = Modifier.weight(1f)
            )
        }

        Spacer(modifier = Modifier.height(20.dp))

        OutlinedTextField(
            value = if (useAccountNumber) accountNumber else email,
            onValueChange = {
                if (useAccountNumber) {
                    accountNumber = it.filter(Char::isDigit).chunked(4).joinToString("-").take(19)
                } else {
                    email = it
                }
            },
            label = { Text(if (useAccountNumber) "Código de 16 dígitos" else "E-mail") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(
                keyboardType = if (useAccountNumber) KeyboardType.Number else KeyboardType.Email,
                imeAction = ImeAction.Next
            ),
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = Accent,
                unfocusedBorderColor = TextSecondary.copy(alpha = 0.3f),
                focusedLabelColor = Accent,
                cursorColor = Accent,
                unfocusedContainerColor = CardBackground,
                focusedContainerColor = CardBackground
            ),
            modifier = Modifier.fillMaxWidth()
        )

        if (!useAccountNumber) {
            Spacer(modifier = Modifier.height(16.dp))

            OutlinedTextField(
                value = password,
                onValueChange = { password = it },
                label = { Text("Senha") },
                singleLine = true,
                visualTransformation = if (passwordVisible) {
                    VisualTransformation.None
                } else {
                    PasswordVisualTransformation()
                },
                trailingIcon = {
                    IconButton(onClick = { passwordVisible = !passwordVisible }) {
                        Icon(
                            imageVector = if (passwordVisible) {
                                Icons.Default.VisibilityOff
                            } else {
                                Icons.Default.Visibility
                            },
                            contentDescription = "Mostrar ou ocultar senha",
                            tint = TextSecondary
                        )
                    }
                },
                keyboardOptions = KeyboardOptions(
                    keyboardType = KeyboardType.Password,
                    imeAction = ImeAction.Done
                ),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = Accent,
                    unfocusedBorderColor = TextSecondary.copy(alpha = 0.3f),
                    focusedLabelColor = Accent,
                    cursorColor = Accent,
                    unfocusedContainerColor = CardBackground,
                    focusedContainerColor = CardBackground
                ),
                modifier = Modifier.fillMaxWidth()
            )
        }

        if (useAccountNumber && createdAccountNumber != null) {
            Spacer(modifier = Modifier.height(12.dp))
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "Seu código: $createdAccountNumber",
                    color = Accent,
                    style = MaterialTheme.typography.bodyMedium,
                    modifier = Modifier.weight(1f)
                )
                OutlinedButton(
                    onClick = {
                        clipboardManager.setText(AnnotatedString(createdAccountNumber.orEmpty()))
                        Toast.makeText(context, "Código copiado", Toast.LENGTH_SHORT).show()
                    },
                    shape = RoundedCornerShape(999.dp)
                ) {
                    Text("Copiar")
                }
            }
        }

        if (error != null) {
            Spacer(modifier = Modifier.height(12.dp))
            Text(
                text = error!!,
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodyMedium
            )
        }

        Spacer(modifier = Modifier.height(32.dp))

        Button(
            onClick = {
                if (useAccountNumber) {
                    viewModel.loginWithNumber(accountNumber)
                } else if (isRegisterMode) {
                    viewModel.register(email, password)
                } else {
                    viewModel.login(email, password)
                }
            },
            enabled = !isLoading && if (useAccountNumber) {
                accountNumber.filter(Char::isDigit).length == 16
            } else {
                email.isNotBlank() && password.isNotBlank()
            },
            modifier = Modifier
                .fillMaxWidth()
                .height(56.dp),
            shape = RoundedCornerShape(999.dp),
            colors = ButtonDefaults.buttonColors(
                containerColor = TextPrimary,
                contentColor = Background
            )
        ) {
            if (isLoading) {
                CircularProgressIndicator(
                    modifier = Modifier.size(24.dp),
                    color = Background,
                    strokeWidth = 2.dp
                )
            } else {
                Text(
                    text = if (useAccountNumber) "ENTRAR COM CÓDIGO" else if (isRegisterMode) "CRIAR CONTA" else "ENTRAR",
                    fontWeight = FontWeight.SemiBold,
                    fontSize = 16.sp
                )
            }
        }

        if (useAccountNumber) {
            Spacer(modifier = Modifier.height(12.dp))
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                OutlinedButton(
                    onClick = {
                        cameraPermissionLauncher.launch(Manifest.permission.CAMERA)
                    },
                    enabled = !isLoading,
                    modifier = Modifier
                        .weight(1f)
                        .height(56.dp),
                    shape = RoundedCornerShape(999.dp),
                    colors = ButtonDefaults.outlinedButtonColors(contentColor = Accent)
                ) {
                    Text("SCAN QR")
                }
                OutlinedButton(
                    onClick = { viewModel.createAnonymousAccount() },
                    enabled = !isLoading,
                    modifier = Modifier
                        .weight(1f)
                        .height(56.dp),
                    shape = RoundedCornerShape(999.dp),
                    colors = ButtonDefaults.outlinedButtonColors(contentColor = Accent)
                ) {
                    Text("GERAR CONTA")
                }
            }
        }

        Spacer(modifier = Modifier.height(24.dp))

        if (!useAccountNumber) {
            Row {
                Text(
                    text = if (isRegisterMode) "Já tem conta? " else "Não tem conta? ",
                    style = MaterialTheme.typography.bodyMedium,
                    color = TextSecondary
                )
                Text(
                    text = if (isRegisterMode) "Entrar" else "Criar conta",
                    style = MaterialTheme.typography.bodyMedium,
                    color = Accent,
                    modifier = Modifier.clickable { isRegisterMode = !isRegisterMode }
                )
            }
        }
    }
}

private class QrScannerBridge(
    private val onScan: (String) -> Unit
) {
    private var consumed = false

    @JavascriptInterface
    fun onQrDetected(rawValue: String) {
        if (consumed) return
        consumed = true
        onScan(rawValue)
    }
}

@Composable
private fun QrScannerDialog(
    onDismiss: () -> Unit,
    onScan: (String) -> Unit
) {
    val html = remember {
        """
        <!doctype html>
        <html>
        <head>
          <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0">
          <style>
            body { margin:0; background:#000; color:#fff; font-family:sans-serif; overflow:hidden; }
            #wrap { position:relative; width:100vw; height:100vh; }
            video { width:100%; height:100%; object-fit:cover; background:#000; }
            #status { position:absolute; top:18px; left:18px; right:18px; background:rgba(0,0,0,0.55); padding:12px 14px; border-radius:16px; font-size:14px; }
            #frame { position:absolute; inset:20% 12%; border:2px solid #C9A84C; border-radius:22px; box-shadow:0 0 0 9999px rgba(0,0,0,0.28); }
          </style>
        </head>
        <body>
        <div id="wrap">
          <video id="video" autoplay playsinline muted></video>
          <div id="frame"></div>
          <div id="status">Aponte a camera para o QR de pareamento</div>
        </div>
        <script>
          const statusEl = document.getElementById('status');
          const video = document.getElementById('video');
          async function start() {
            if (!('BarcodeDetector' in window)) {
              statusEl.textContent = 'Scanner nao suportado neste dispositivo';
              return;
            }
            const detector = new BarcodeDetector({ formats: ['qr_code'] });
            try {
              const stream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: 'environment' }, audio: false });
              video.srcObject = stream;
              await video.play();
              const tick = async () => {
                try {
                  const codes = await detector.detect(video);
                  if (codes && codes.length > 0 && codes[0].rawValue) {
                    statusEl.textContent = 'QR detectado';
                    if (window.AndroidBridge && window.AndroidBridge.onQrDetected) {
                      window.AndroidBridge.onQrDetected(codes[0].rawValue);
                    }
                    stream.getTracks().forEach(t => t.stop());
                    return;
                  }
                } catch (e) {}
                requestAnimationFrame(tick);
              };
              requestAnimationFrame(tick);
            } catch (e) {
              statusEl.textContent = 'Nao foi possivel abrir a camera';
            }
          }
          start();
        </script>
        </body>
        </html>
        """.trimIndent()
    }

    Dialog(onDismissRequest = onDismiss) {
        Surface(
            shape = RoundedCornerShape(24.dp),
            color = Background,
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(16.dp)
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = "Escanear QR",
                        style = MaterialTheme.typography.titleLarge,
                        color = TextPrimary
                    )
                    IconButton(onClick = onDismiss) {
                        Text("Fechar", color = Accent)
                    }
                }
                Spacer(modifier = Modifier.height(12.dp))
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(420.dp)
                        .background(CardBackground, RoundedCornerShape(18.dp))
                ) {
                    AndroidView(
                        factory = { context ->
                            WebView(context).apply {
                                settings.javaScriptEnabled = true
                                settings.mediaPlaybackRequiresUserGesture = false
                                webChromeClient = WebChromeClient()
                                webViewClient = WebViewClient()
                                addJavascriptInterface(QrScannerBridge(onScan), "AndroidBridge")
                                loadDataWithBaseURL(null, html, "text/html", "UTF-8", null)
                            }
                        },
                        modifier = Modifier.fillMaxSize()
                    )
                }
            }
        }
    }
}

@Composable
private fun ModeChip(
    label: String,
    selected: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    Button(
        onClick = onClick,
        modifier = modifier.height(46.dp),
        shape = RoundedCornerShape(999.dp),
        colors = ButtonDefaults.buttonColors(
            containerColor = if (selected) TextPrimary else CardBackground,
            contentColor = if (selected) Background else TextSecondary
        )
    ) {
        Text(label)
    }
}
