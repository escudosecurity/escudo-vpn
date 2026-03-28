package com.escudo.vpn.service

import android.Manifest
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.ConnectivityManager
import android.net.VpnService
import android.net.wifi.WifiManager
import android.os.Build
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import com.escudo.vpn.EscudoApp
import com.escudo.vpn.data.prefs.SecurePrefs
import com.escudo.vpn.ui.MainActivity

class WifiMonitor : BroadcastReceiver() {

    companion object {
        private const val TAG = "WifiMonitor"
        private const val WIFI_PROTECT_NOTIFICATION_ID = 2
    }

    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != WifiManager.NETWORK_STATE_CHANGED_ACTION) return

        val connectivityManager = context.getSystemService(ConnectivityManager::class.java) ?: return
        val activeNetwork = connectivityManager.activeNetwork ?: return
        val capabilities = connectivityManager.getNetworkCapabilities(activeNetwork) ?: return
        if (!capabilities.hasTransport(android.net.NetworkCapabilities.TRANSPORT_WIFI)) return

        val securePrefs = SecurePrefs(context)

        if (!securePrefs.isAutoProtectEnabled()) return
        if (!securePrefs.isLoggedIn()) return

        val ssid = WifiSsidResolver.getCurrentSsid(context)
        if (ssid == null) {
            Log.d(TAG, "Unable to determine SSID, skipping auto-protect")
            return
        }

        val trustedNetworks = securePrefs.getTrustedNetworks()
        if (trustedNetworks.contains(ssid)) {
            Log.d(TAG, "Connected to trusted network")
            return
        }

        // Check if VPN is already active
        if (VpnService.prepare(context) == null) {
            // VPN permission granted — check if service is already running
            // by seeing if connection state would indicate active tunnel.
            // Start the VPN service to trigger auto-connect.
            Log.d(TAG, "Untrusted network detected. Starting VPN.")
            val serviceIntent = Intent(context, EscudoVpnService::class.java).apply {
                putExtra(EscudoVpnService.EXTRA_AUTO_CONNECT, true)
            }
            ContextCompat.startForegroundService(context, serviceIntent)
            showNotification(context, ssid)
        } else {
            Log.d(TAG, "VPN permission not granted, cannot auto-connect")
        }
    }

    private fun showNotification(context: Context, ssid: String) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            val hasPermission = ContextCompat.checkSelfPermission(
                context,
                Manifest.permission.POST_NOTIFICATIONS
            ) == PackageManager.PERMISSION_GRANTED
            if (!hasPermission) return
        }

        val intent = Intent(context, MainActivity::class.java)
        val pendingIntent = PendingIntent.getActivity(
            context,
            0,
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val notification = NotificationCompat.Builder(context, EscudoApp.VPN_CHANNEL_ID)
            .setSmallIcon(android.R.drawable.ic_lock_lock)
            .setContentTitle("Rede publica detectada")
            .setContentText("Rede nao confiavel detectada. Escudo ativado automaticamente.")
            .setContentIntent(pendingIntent)
            .setAutoCancel(true)
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .build()

        NotificationManagerCompat.from(context)
            .notify(WIFI_PROTECT_NOTIFICATION_ID, notification)
    }
}
