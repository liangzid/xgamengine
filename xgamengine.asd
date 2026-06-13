;; ======================================================================
;; XGAMENGINE.ASD
;;
;; 1. ASDF system definition for the xgamengine — an AI-text-based game engine.
;; 2. Declares all dependencies, source modules, and test entry points.
;; 3. (2025-06-13) Initial creation for MVP Phase 0.
;;
;;     Author: liangzid
;;     Copyright © 2025, all rights reserved.
;;     Created: 13 June 2025
;; ======================================================================
(asdf:defsystem "xgamengine"
  :description "AI-text-based game engine for xianxia simulation."
  :author "liangzid"
  :license "TBD"
  :version "0.1.0"
  :depends-on ("dexador"               ; HTTP client
               "shasht"                ; JSON parsing & streaming
               "cl-ppcre"              ; regex for template rendering
               "str"                   ; string utilities
               "fiveam")               ; testing framework
  :serial t
  :components ((:file "src/package")
               (:file "src/json")
               (:file "src/llm/client")
               (:file "src/llm/stream")
               (:file "src/prompt/loader")
               (:file "src/prompt/builder")
               (:file "src/state/state")
               (:file "src/memory/short-term")
               (:file "src/engine"))
  :in-order-to ((asdf:test-op (asdf:test-op "xgamengine/tests"))))

(asdf:defsystem "xgamengine/tests"
  :description "Test suite for xgamengine."
  :author "liangzid"
  :license "TBD"
  :depends-on ("xgamengine" "fiveam")
  :serial t
  :components ((:file "tests/suite")
               (:file "tests/integration-test"))
  :perform (asdf:test-op (op sys)
             (declare (ignore op sys))
             (uiop:symbol-call :xgamengine-tests :run-tests)))
