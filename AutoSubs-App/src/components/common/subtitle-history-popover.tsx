import { useState, useEffect } from "react"
import { useTranslation } from "react-i18next"
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover"
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command"
import { listSubtitleDocuments, readSubtitleDocument, type SubtitleDocumentListItem } from "@/utils/file-utils"
import { useSubtitleDocument } from "@/contexts/SubtitleDocumentContext"

interface SubtitleHistoryPopoverProps {
  trigger: React.ReactNode
  onSubtitleDocumentOpen?: () => void
  align?: "start" | "center" | "end"
}

export function SubtitleHistoryPopover({ trigger, onSubtitleDocumentOpen, align = "center" }: SubtitleHistoryPopoverProps) {
  const { t, i18n } = useTranslation()
  const [open, setOpen] = useState(false)
  const [subtitleDocuments, setSubtitleDocuments] = useState<SubtitleDocumentListItem[]>([])
  const [loading, setLoading] = useState(false)
  const [hasLoaded, setHasLoaded] = useState(false)
  const { setSubtitles, setSpeakers, setCurrentSubtitleDocumentFilename } = useSubtitleDocument()

  useEffect(() => {
    loadSubtitleDocuments()
  }, [])

  useEffect(() => {
    if (open) {
      loadSubtitleDocuments()
    }
  }, [open])

  const loadSubtitleDocuments = async () => {
    setLoading(true)
    try {
      setSubtitleDocuments(await listSubtitleDocuments())
      setHasLoaded(true)
    } catch (error) {
      console.error('Failed to load subtitle documents:', error)
    } finally {
      setLoading(false)
    }
  }

  const subtitleDocumentDateLocale = i18n.resolvedLanguage || i18n.language || undefined

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        {trigger}
      </PopoverTrigger>
      <PopoverContent className="w-80 p-0" align={align}>
        <Command>
          <CommandInput placeholder={t("titlebar.subtitleHistory.searchPlaceholder")} />
          <CommandList>
            {loading && subtitleDocuments.length === 0 && !hasLoaded ? (
              <div className="py-4 text-center text-sm text-muted-foreground">
                {t("titlebar.subtitleHistory.loading")}
              </div>
            ) : subtitleDocuments.length === 0 ? (
              <CommandEmpty>{t("titlebar.subtitleHistory.empty")}</CommandEmpty>
            ) : (
              <CommandGroup>
                {subtitleDocuments.map((subtitleDocument) => (
                  <CommandItem
                    key={subtitleDocument.filename}
                    value={`${subtitleDocument.displayName} ${subtitleDocument.filename}`}
                    className="cursor-pointer"
                    onSelect={async () => {
                      try {
                        const subtitleDocumentData = await readSubtitleDocument(subtitleDocument.filename)
                        if (subtitleDocumentData) {
                          setSubtitles(subtitleDocumentData.segments || [])
                          setSpeakers(subtitleDocumentData.speakers || [])
                          setCurrentSubtitleDocumentFilename(subtitleDocument.filename)
                          onSubtitleDocumentOpen?.()
                        }
                      } catch (error) {
                        console.error('Failed to load subtitle document:', error)
                      }
                      setOpen(false)
                    }}
                  >
                    <div className="flex flex-col items-start gap-1">
                      <span className="text-sm font-medium">
                        {subtitleDocument.displayName}
                      </span>
                      <span className="text-xs text-muted-foreground">
                        {subtitleDocument.createdAt.toLocaleDateString(subtitleDocumentDateLocale, {
                          month: 'short',
                          day: 'numeric',
                          year: 'numeric',
                          hour: 'numeric',
                          minute: '2-digit'
                        })}
                      </span>
                    </div>
                  </CommandItem>
                ))}
              </CommandGroup>
            )}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  )
}
