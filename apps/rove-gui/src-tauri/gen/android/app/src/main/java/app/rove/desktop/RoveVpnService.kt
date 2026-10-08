package app.rove.desktop

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Intent
import android.net.VpnService
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.ParcelFileDescriptor
import android.system.OsConstants
import androidx.core.app.NotificationCompat
import org.json.JSONArray
import java.lang.ref.WeakReference
import java.net.InetAddress

/** Only OS consent, foreground lifetime and TUN creation live in Kotlin. */
object RoveVpnController {
    private var activity = WeakReference<MainActivity>(null)
    @Volatile var service: RoveVpnService? = null
    // 0 waiting; 1 service ready; -1 denied/revoked/unavailable.
    @Volatile private var permissionState = -1

    fun attach(owner: MainActivity) { activity = WeakReference(owner) }
    fun requestPermission() {
        val owner = activity.get() ?: return
        // A Rust request may time out while the system dialog is still open.
        // Reuse that pending consent rather than stacking a second dialog.
        if (permissionState == 0) return
        permissionState = 0
        owner.runOnUiThread {
            try {
                val consent = VpnService.prepare(owner)
                if (consent == null) permissionResult(true)
                else owner.requestRoveVpn(consent)
            } catch (_: Exception) { permissionState = -1 }
        }
    }
    fun permissionResult(granted: Boolean) {
        val owner = activity.get()
        if (!granted || owner == null) { permissionState = -1; return }
        try {
            val intent = Intent(owner, RoveVpnService::class.java)
            if (Build.VERSION.SDK_INT >= 26) owner.startForegroundService(intent)
            else owner.startService(intent)
        } catch (_: Exception) { permissionState = -1 }
    }
    fun started(value: RoveVpnService) { service = value; permissionState = 1 }
    fun stopped(value: RoveVpnService) {
        if (service === value) { service = null; permissionState = -1 }
    }
    fun permissionStatus(): Int {
        val current = service
        if (permissionState == 1 && (current == null || VpnService.prepare(current) != null)) return -1
        return permissionState
    }
    fun establish(plan: String): Int = service?.establishPlan(plan)
        ?: throw IllegalStateException("Native VPN foreground service is not ready")
    fun close() { service?.shutdown() }
}

class RoveVpnService : VpnService() {
    private var tunnel: ParcelFileDescriptor? = null
    private var closing = false

    override fun onCreate() {
        super.onCreate()
        val manager = getSystemService(NotificationManager::class.java)
        if (Build.VERSION.SDK_INT >= 26) {
            manager.createNotificationChannel(NotificationChannel("rove-network", "Rove network", NotificationManager.IMPORTANCE_LOW))
        }
        val open = PendingIntent.getActivity(this, 0, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        val notification = NotificationCompat.Builder(this, "rove-network")
            .setSmallIcon(android.R.drawable.stat_sys_upload_done)
            .setContentTitle("Rove")
            .setContentText(if (resources.configuration.locales[0].language == "zh") "网络服务运行中" else "Network service is active")
            .setContentIntent(open).setOngoing(true).build()
        startForeground(43190, notification)
        RoveVpnController.started(this)
        // Consent without a successful start must not leave an idle service.
        Handler(Looper.getMainLooper()).postDelayed({ synchronized(this) {
            if (tunnel == null) shutdown()
        } }, 60_000)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int = START_NOT_STICKY

    @Synchronized
    fun establishPlan(raw: String): Int {
        check(!closing && prepare(this) == null) { "Native VPN permission is unavailable" }
        require(raw.length <= 32768) { "VPN plan exceeds limit" }
        val plan = JSONArray(raw)
        require(plan.length() in 1..64) { "Invalid VPN plan size" }
        val builder = Builder().setSession("Rove").setMtu(1280).setBlocking(false)
            .allowFamily(OsConstants.AF_INET6)
        // Never install an Internet default route, DNS override, or a rule
        // excluding Rove itself: Rove agent traffic must use this VPN too.
        for (i in 0 until plan.length()) {
            val item = plan.getJSONObject(i)
            val prefix = item.getInt("prefix")
            require(prefix in 1..30) { "Invalid overlay prefix" }
            builder.addAddress(ipv4(item.getString("address")), prefix)
            builder.addRoute(ipv4(item.getString("network")), prefix)
        }
        if (Build.VERSION.SDK_INT >= 29) builder.setMetered(false)
        val next = builder.establish() ?: throw IllegalStateException("Native VPN authorization was revoked")
        val fd = try { ParcelFileDescriptor.dup(next.fileDescriptor).detachFd() }
        catch (error: Exception) { next.close(); throw error }
        val previous = tunnel
        tunnel = next
        // The detached duplicate must reach Rust even if closing the old Java
        // owner reports an error; otherwise its fd would leak across JNI.
        runCatching { previous?.close() }
        return fd // Rust assumes sole ownership of this duplicate.
    }

    private fun ipv4(raw: String): InetAddress {
        val parts = raw.split('.')
        require(parts.size == 4 && parts.all {
            it.isNotEmpty() && it.all { c -> c in '0'..'9' } &&
                it.toIntOrNull()?.let { octet -> octet in 0..255 } == true
        }) { "Invalid IPv4 literal" }
        return InetAddress.getByAddress(parts.map { it.toInt().toByte() }.toByteArray())
    }

    @Synchronized
    fun shutdown() {
        if (closing) return
        closing = true
        RoveVpnController.stopped(this)
        try { tunnel?.close() } finally {
            tunnel = null
            stopForeground(STOP_FOREGROUND_REMOVE)
            stopSelf()
        }
    }
    override fun onRevoke() { shutdown(); super.onRevoke() }
    override fun onDestroy() { shutdown(); super.onDestroy() }
}
