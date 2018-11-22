(
    (
        \y ((\x (y (x x))) (\x (y (x x)))) # Y Combinator
        \self (\lower upper target (
            (\mid (
                $if
                    ($lt ($sub upper lower) 0.0000000001)
                    mid
                    ($if
                        ($lt ($mul mid mid) target)
                        (self mid upper target)
                        (self lower mid target)
                    )
            )) ($div ($add upper lower) 2)
        ))
    ) 0.0 5.0 5.0
)
