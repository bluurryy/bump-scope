inspect_asm::alloc_u8::try_up_a:
	push rbx
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	cmp qword ptr [rcx + 8], rax
	je .LBB_3
	mov rdx, rax
	and rdx, -4
	add rdx, 4
	mov qword ptr [rcx], rdx
.LBB_2:
	mov byte ptr [rax], sil
	pop rbx
	ret
.LBB_3:
	mov ebx, esi
	call bump_scope::bump_scope::BumpScope<A,_,_>::do_alloc_sized_in_another_chunk
	mov esi, ebx
	test rax, rax
	jne .LBB_2
	xor eax, eax
	pop rbx
	ret