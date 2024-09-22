inspect_asm::alloc_vec3::try_up:
	push rbx
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov rdx, qword ptr [rcx + 8]
	add rax, 3
	and rax, -4
	sub rdx, rax
	cmp rdx, 11
	jbe .LBB0_0
	lea rdx, [rax + 12]
	mov qword ptr [rcx], rdx
	mov ecx, dword ptr [rsi + 8]
	mov dword ptr [rax + 8], ecx
	mov rcx, qword ptr [rsi]
	mov qword ptr [rax], rcx
	test rax, rax
	je .LBB0_1
	pop rbx
	ret
.LBB0_0:
	mov rbx, rsi
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_sized_in_another_chunk
	test rax, rax
	je .LBB0_1
	mov ecx, dword ptr [rbx + 8]
	mov dword ptr [rax + 8], ecx
	mov rcx, qword ptr [rbx]
	mov qword ptr [rax], rcx
	pop rbx
	ret
.LBB0_1:
	xor eax, eax
	pop rbx
	ret
