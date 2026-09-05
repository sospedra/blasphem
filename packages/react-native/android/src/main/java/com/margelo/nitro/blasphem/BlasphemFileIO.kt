package com.margelo.nitro.blasphem

import android.content.Context
import com.margelo.nitro.NitroModules

internal object BlasphemFileIO {
  fun context(): Context = NitroModules.applicationContext
    ?: throw java.io.IOException("BLASPHEM_FETCH_FAILED: An application context is required")
}
