#|======================================================================
LOADER.LISP

1. Prompt template loader — reads .md templates from disk and renders {{variables}}.
2. Called by: prompt/builder.lisp → loader.lisp → disk file read via cl-ppcre.
3. (2025-06-13) Phase 2: load from disk with variable interpolation.

    Author: liangzid
    Copyright © 2025, all rights reserved.
    Created: 13 June 2025
======================================================================|#
(in-package :xgamengine)

;; ---- Configuration ----

(defvar *template-dir* nil
  "Directory where prompt template .md files are stored.
Defaults to the 'templates/' directory relative to the xgame project root,
which is one level above the xgamengine submodule root.")

(defun resolve-template-dir ()
  "Return the resolved template directory path.
On first call, computes the default path if *template-dir* is NIL."
  (or *template-dir*
      (setf *template-dir*
            (merge-pathnames "../templates/"
                             (asdf:system-source-directory :xgamengine)))))

;; ---- Template Loading ----

(defun template-file-path (name)
  "Return the full pathname for template NAME.
NAME is a string without the '.md' extension."
  (let ((dir (resolve-template-dir))
        (filename (concatenate 'string name ".md")))
    (merge-pathnames filename dir)))

(defun load-template (name)
  "Load a prompt template from disk.
NAME is the template name without '.md' extension, e.g. \"world-rules\".
Returns the file contents as a string.
Signals an error if the file does not exist."
  (let ((path (template-file-path name)))
    (unless (uiop:file-exists-p path)
      (error "Template file not found: ~a" path))
    (uiop:read-file-string path)))

(defun load-all-templates (&rest names)
  "Load multiple templates and return them as an alist of (name . content)."
  (mapcar (lambda (name)
            (cons name (load-template name)))
          names))

;; ---- Variable Interpolation ----

(defun render-template (template-string bindings)
  "Replace {{variable}} placeholders in TEMPLATE-STRING with values from BINDINGS.
BINDINGS is an alist of (variable-name . value), where both are strings.
Variables not found in bindings are left as-is (with a warning to *error-output*).

Example:
  (render-template \"Hello {{name}}!\" '((\"name\" . \"World\")))
  => \"Hello World!\""
  (cl-ppcre:regex-replace-all
   "\\{\\{(.+?)\\}\\}"
   template-string
   (lambda (match var-name)
     (declare (ignore match))
     (let* ((key (string-trim '(#\Space) var-name))
            (binding (assoc key bindings :test #'string=)))
       (if binding
           (cdr binding)
           (progn
             (format *error-output*
                     "Warning: template variable '{{~a}}' not found in bindings.~%"
                     key)
             (concatenate 'string "{{" key "}}")))))
   :simple-calls t))

(defun render-template-file (name bindings)
  "Load a template file and render it with BINDINGS in one step.
BINDINGS is an alist of (\"var\" . \"value\") strings."
  (let ((raw (load-template name)))
    (render-template raw bindings)))
