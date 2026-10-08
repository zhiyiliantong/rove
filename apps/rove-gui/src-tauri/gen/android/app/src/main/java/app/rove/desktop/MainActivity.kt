package app.rove.desktop

import android.os.Bundle
import android.view.View
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.activity.result.contract.ActivityResultContracts
import android.content.Intent
import android.app.Activity

class MainActivity : TauriActivity() {
  private external fun initializeTls(context: android.content.Context)
  private external fun initializeVpn(controller: RoveVpnController)
  private val vpnConsent = registerForActivityResult(ActivityResultContracts.StartActivityForResult()) {
    RoveVpnController.permissionResult(it.resultCode == Activity.RESULT_OK)
  }
  fun requestRoveVpn(intent: Intent) { vpnConsent.launch(intent) }

  override fun onCreate(savedInstanceState: Bundle?) {
    System.loadLibrary("rove_gui")
    initializeTls(applicationContext)
    RoveVpnController.attach(this)
    initializeVpn(RoveVpnController)
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    // Keep the entire WebView viewport within usable bounds. CSS env() alone
    // does not report system bars consistently across Android WebView versions.
    // Consume these insets once here so fixed Vue headers/tabs and dialogs do
    // not overlap system controls or double-apply a second safe-area padding.
    val content = findViewById<View>(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(content) { view, windowInsets ->
      val safe = windowInsets.getInsets(
        WindowInsetsCompat.Type.systemBars() or
          WindowInsetsCompat.Type.displayCutout() or
          WindowInsetsCompat.Type.ime()
      )
      view.setPadding(safe.left, safe.top, safe.right, safe.bottom)
      WindowInsetsCompat.CONSUMED
    }
    ViewCompat.requestApplyInsets(content)
  }
}
